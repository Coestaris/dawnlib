use crate::gl::context::Context;
use crate::passes::chain::RenderChain;
use crate::passes::events::RenderPassEvent;
use crate::passes::pipeline::RenderPipeline;
use crate::passes::result::RenderResult;
use crate::passes::ChainExecuteCtx;
use crate::renderer::backend::RendererBackendTrait;
use crate::renderer::monitor::RendererMonitorTrait;
use crate::renderer::RendererConfig;
use crate::renderer::{CustomRenderer, RendererBackend};
use crate::renderer::{
    DataStreamFrame, InputEvent, OutputEvent, PassEventTrait, RendezvousTrait, WindowConfig,
};
use crossbeam_channel::{Receiver, Sender};
use log::{info, warn};
use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use thiserror::Error;
use triple_buffer::Output;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize, Size};
use winit::error::EventLoopError;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("Event loop error: {0}")]
    EventLoopError(#[from] EventLoopError),
}

pub(crate) struct Application<P, C, E, R, BF, AF>
where
    E: PassEventTrait,
    P: RendererMonitorTrait,
    C: RenderChain<E>,
    R: CustomRenderer<C, E>,
    BF: RendezvousTrait,
    AF: RendezvousTrait,
{
    pipeline: Option<RenderPipeline<C, E>>,

    config: WindowConfig,
    frame_index: usize,

    backend_config: RendererConfig,
    external_stop: Arc<AtomicBool>,
    renderer: R,
    before_frame: BF,
    after_frame: AF,
    monitor: P,

    // In/Out queues
    renderer_in: Receiver<RenderPassEvent<E>>,
    view_in: Receiver<OutputEvent>,
    data_stream: Output<DataStreamFrame>,
    input_out: Sender<InputEvent>,

    // The backend and window must be dropped after the pipeline
    backend: Option<RendererBackend<E>>,
    window: Option<Window>,
}

impl<P, C, E, R, BF, AF> Application<P, C, E, R, BF, AF>
where
    E: PassEventTrait,
    P: RendererMonitorTrait,
    C: RenderChain<E>,
    R: CustomRenderer<C, E>,
    BF: RendezvousTrait,
    AF: RendezvousTrait,
{
    // God forgive me for this abomination.
    pub(crate) fn new(
        config: WindowConfig,
        backend_config: RendererConfig,
        monitor: P,
        renderer: R,
        before_frame: BF,
        after_frame: AF,
        external_stop: Arc<AtomicBool>,
        renderer_in: Receiver<RenderPassEvent<E>>,
        output_in: Receiver<OutputEvent>,
        data_stream: Output<DataStreamFrame>,
        input_out: Sender<InputEvent>,
    ) -> Result<Self, ApplicationError> {
        Ok(Application {
            renderer,
            before_frame,
            after_frame,
            config,
            window: None,
            external_stop,
            pipeline: None,
            monitor,
            renderer_in,
            view_in: output_in,
            data_stream,
            backend_config,
            frame_index: 0,
            backend: None,
            input_out,
        })
    }
}

impl<P, C, E, R, BF, AF> ApplicationHandler for Application<P, C, E, R, BF, AF>
where
    E: PassEventTrait,
    P: RendererMonitorTrait,
    C: RenderChain<E>,
    R: CustomRenderer<C, E>,
    BF: RendezvousTrait,
    AF: RendezvousTrait,
{
    fn new_events(&mut self, event_loop: &ActiveEventLoop, _cause: StartCause) {
        if let (Some(window), Some(backend), Some(pipeline)) = (
            self.window.as_ref(),
            self.backend.as_mut(),
            self.pipeline.as_mut(),
        ) {
            // This has no sense if no synchronization is disabled,
            // but if it is, it is a good idea to process all the events between frames.
            // Here, in the meanwhile, the Main thread will copy the renderables to us,
            // so we have some time to handle events.
            // It also guarantees that all the events the user produced will be processed
            // before the next frame.
            self.monitor.events_start();
            for event in self.renderer_in.try_iter() {
                pipeline.dispatch(event);
            }
            self.monitor.events_stop();

            if self.external_stop.load(std::sync::atomic::Ordering::SeqCst) {
                event_loop.exit();
                info!("External stop requested, exiting renderer loop");

                self.before_frame.unlock();
                self.after_frame.unlock();
                return;
            }

            // Meet with the Main thread
            self.before_frame.wait();
            let _ = backend.before_frame();
            self.renderer.before_frame(window, backend);

            // Process View. Usually this will produce input events
            self.monitor.view_start();
            for event in self.view_in.try_iter() {
                match event {
                    OutputEvent::ChangeTitle(title) => {
                        window.set_title(&title);
                    }
                    OutputEvent::ChangeWindowSize(size) => {
                        window.set_min_inner_size(Some(Size::Logical(LogicalSize::new(
                            size.x as f64,
                            size.y as f64,
                        ))));
                    }
                    OutputEvent::ChangeResizable(resizable) => {
                        window.set_resizable(resizable);
                    }
                    OutputEvent::ChangeDecorations(decorations) => {
                        window.set_decorations(decorations);
                    }
                    OutputEvent::ChangeFullscreen(fullscreen) => {
                        if fullscreen {
                            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                        } else {
                            window.set_fullscreen(None);
                        }
                    }
                    OutputEvent::ChangeIcon(icon) => {
                        window.set_window_icon(icon.clone());
                        #[cfg(target_os = "windows")]
                        {
                            use winit::platform::windows::WindowExtWindows;
                            window.set_taskbar_icon(icon);
                        }
                    }
                    OutputEvent::ChangeCursor(cursor) => {
                        if let Some(cursor) = cursor {
                            window.set_cursor(cursor.clone());
                            window.set_cursor_visible(true);
                        } else {
                            window.set_cursor_visible(false);
                        }
                    }
                }
            }
            self.monitor.view_stop();
        } else {
            // Unlikely, but if we are not ready yet,
            // just wait for the Main thread to not freeze it.
            self.before_frame.wait();
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(not(web_platform))]
        let mut window_attributes = WindowAttributes::default()
            .with_title(self.config.title.clone())
            .with_inner_size(Size::Logical(LogicalSize::new(
                self.config.dimensions.x as f64,
                self.config.dimensions.y as f64,
            )))
            .with_resizable(self.config.resizable)
            .with_visible(true)
            .with_window_icon(self.config.icon.clone())
            .with_decorations(self.config.decorations);

        if self.config.fullscreen {
            window_attributes =
                window_attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        if let Some(cursor) = &self.config.cursor {
            window_attributes = window_attributes.with_cursor(cursor.clone());
        }

        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::WindowAttributesExtWindows;
            window_attributes = window_attributes.with_taskbar_icon(self.config.icon.clone());
        }

        let (window, context) =
            match Context::create_contextual_window(window_attributes, event_loop) {
                Ok(pair) => pair,
                Err(err) => {
                    eprintln!("error creating window: {err}");
                    event_loop.exit();
                    panic!("Failed to create window");
                }
            };
        self.window = Some(window);

        if let Some(_) = &self.config.cursor {
            self.window.as_ref().unwrap().set_cursor_visible(true);
        } else {
            self.window.as_ref().unwrap().set_cursor_visible(false);
        }

        event_loop.set_control_flow(ControlFlow::Poll);

        self.backend =
            Some(RendererBackend::<E>::new(self.backend_config.clone(), context).unwrap());

        let backend = self.backend.as_mut().unwrap();
        let backend_static = unsafe {
            // SAFETY: We are the only thread that can access the backend.
            // and it's guaranteed to pipeline be dropped before the backend.
            mem::transmute::<&mut RendererBackend<E>, &'static mut RendererBackend<E>>(backend)
        };
        self.pipeline = Some(RenderPipeline::new(
            self.renderer
                .spawn_chain(&self.window.as_ref().unwrap(), backend_static)
                .unwrap(),
        ));

        self.input_out
            .send(InputEvent(WindowEvent::Resized(PhysicalSize::new(
                self.config.dimensions.x as u32,
                self.config.dimensions.y as u32,
            ))))
            .unwrap();

        // Notify the monitor about the pass names
        let pass_names = self.pipeline.as_ref().unwrap().get_names().clone();
        self.monitor.set_pass_names(&pass_names);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let (Some(window), Some(backend), Some(pipeline)) = (
            self.window.as_ref(),
            self.backend.as_mut(),
            self.pipeline.as_mut(),
        ) {
            // Do not spam the channel with redraw requests.
            // RedrawRequested events are sent every frame in about_to_wait.
            if !matches!(event, WindowEvent::RedrawRequested) {
                // Receiver may be dead. We don't care.
                let _ = self.input_out.send(InputEvent(event.clone()));
                self.renderer.on_window_event(window, backend, &event);
            }

            match event {
                WindowEvent::Resized(size) => {
                    backend
                        .resize(glam::UVec2::new(
                            size.width.max(1) as u32,
                            size.height.max(1) as u32,
                        ))
                        .unwrap();
                }
                WindowEvent::CloseRequested => {
                    info!("Window close requested");

                    // Tell other threads to stop
                    self.external_stop
                        .store(true, std::sync::atomic::Ordering::SeqCst);

                    event_loop.exit();

                    self.before_frame.unlock();
                    self.after_frame.unlock();
                }
                WindowEvent::RedrawRequested => {
                    // Notify that you're about to draw.
                    window.pre_present_notify();

                    self.monitor.render_start();

                    self.renderer.before_render(window, backend);

                    let frame = self.data_stream.read();
                    if frame.epoch != self.frame_index {
                        warn!(
                            "Renderer is out of sync! Expected epoch {}, got {}",
                            self.frame_index, frame.epoch
                        );
                        self.frame_index = frame.epoch;
                        self.frame_index += 1;
                    } else {
                        self.frame_index += 1;
                    }

                    // Render the frame
                    let mut ctx = ChainExecuteCtx::new(frame, backend);
                    let pass_result = pipeline.execute(&mut ctx);
                    let durations = mem::take(&mut ctx.durations);
                    drop(ctx);

                    self.renderer.after_render(window, backend);

                    // Do not include after frame in the monitoring, because it usually synchronizes
                    // the rendered frame with the OS by swapping buffer, that usually is synchronized
                    // with the refresh rate of the display. So this will not be informative.
                    self.monitor.render_stop(pass_result, &durations);
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let (Some(window), Some(backend)) = (self.window.as_ref(), self.backend.as_mut()) {
            // Some platforms wants to be notified before presenting the frame.
            // So request a redraw just before waiting for new events.
            window.request_redraw();

            let _ = backend.after_frame();
            self.renderer.after_frame(window, backend);

            // Meet with the Main thread again.
            self.after_frame.wait();
        }
    }
}
