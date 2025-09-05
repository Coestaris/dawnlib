use crate::gl::context::Context;
use crate::passes::chain::RenderChain;
use crate::passes::events::RenderPassEvent;
use crate::passes::pipeline::RenderPipeline;
use crate::passes::result::RenderResult;
use crate::passes::ChainExecuteCtx;
use crate::renderer::backend::RendererBackendTrait;
use crate::renderer::monitor::RendererMonitorTrait;
use crate::renderer::{
    DataStreamFrame, InputEvent, OutputEvent, PassEventTrait, RendezvousTrait, WindowConfig,
};
use crate::renderer::{RenderChainConstructor, RendererConfig};
use crate::renderer::{RendererBackend, RendererError};
use crossbeam_channel::{Receiver, Sender};
use dawn_util::rendezvous::Rendezvous;
use log::{info, warn};
use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use thiserror::Error;
use triple_buffer::Output;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, Size};
use winit::error::EventLoopError;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("Event loop error: {0}")]
    EventLoopError(#[from] EventLoopError),
}

pub(crate) struct Application<P, C, E>
where
    E: PassEventTrait,
    P: RendererMonitorTrait,
    C: RenderChain<E>,
{
    window: Option<Window>,
    chain: Option<RenderPipeline<C, E>>,
    backend: Option<RendererBackend<E>>,

    config: WindowConfig,
    frame_index: usize,

    backend_config: RendererConfig,
    external_stop: Arc<AtomicBool>,
    constructor: Box<dyn RenderChainConstructor<C, E>>,
    before_frame: Box<dyn RendezvousTrait>,
    after_frame: Box<dyn RendezvousTrait>,
    monitor: P,

    // In/Out queues
    renderer_in: Receiver<RenderPassEvent<E>>,
    view_in: Receiver<OutputEvent>,
    data_stream: Output<DataStreamFrame>,
    input_out: Sender<InputEvent>,
}

impl<P, C, E> Application<P, C, E>
where
    E: PassEventTrait,
    P: RendererMonitorTrait,
    C: RenderChain<E>,
{
    // God forgive me for this abomination.
    pub(crate) fn new(
        config: WindowConfig,
        backend_config: RendererConfig,
        monitor: P,
        constructor: impl RenderChainConstructor<C, E>,
        before_frame: impl RendezvousTrait,
        after_frame: impl RendezvousTrait,
        external_stop: Arc<AtomicBool>,
        renderer_in: Receiver<RenderPassEvent<E>>,
        output_in: Receiver<OutputEvent>,
        data_stream: Output<DataStreamFrame>,
        input_out: Sender<InputEvent>,
    ) -> Result<Self, ApplicationError> {
        Ok(Application {
            constructor: Box::new(constructor),
            before_frame: Box::new(before_frame),
            after_frame: Box::new(after_frame),
            config,
            window: None,
            external_stop,
            chain: None,
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

impl<P, C, E> ApplicationHandler for Application<P, C, E>
where
    E: PassEventTrait,
    P: RendererMonitorTrait,
    C: RenderChain<E>,
{
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        // This has no sense if no synchronization is disabled,
        // but if it is, it is a good idea to process all the events between frames.
        // Here, in the meanwhile, the Main thread will copy the renderables to us,
        // so we have some time to handle events.
        // It also guarantees that all the events the user produced will be processed
        // before the next frame.
        self.monitor.events_start();
        for event in self.renderer_in.try_iter() {
            self.chain.as_mut().unwrap().dispatch(event);
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

        // Process View. Usually this will produce input events
        self.monitor.view_start();
        for event in self.view_in.try_iter() {
            match event {
                OutputEvent::ChangeTitle(title) => {
                    self.window.as_ref().unwrap().set_title(&title);
                }
                OutputEvent::ChangeWindowSize(size) => {
                    self.window
                        .as_ref()
                        .unwrap()
                        .set_min_inner_size(Some(Size::Logical(LogicalSize::new(
                            size.x as f64,
                            size.y as f64,
                        ))));
                }
                OutputEvent::ChangeResizable(resizable) => {
                    self.window.as_ref().unwrap().set_resizable(resizable);
                }
                OutputEvent::ChangeDecorations(decorations) => {
                    self.window.as_ref().unwrap().set_decorations(decorations);
                }
                OutputEvent::ChangeFullscreen(fullscreen) => {
                    if fullscreen {
                        self.window
                            .as_ref()
                            .unwrap()
                            .set_fullscreen(Some(Fullscreen::Borderless(None)));
                    } else {
                        self.window.as_ref().unwrap().set_fullscreen(None);
                    }
                }
                OutputEvent::ChangeIcon(icon) => {
                    self.window.as_ref().unwrap().set_window_icon(icon.clone());
                    #[cfg(target_os = "windows")]
                    self.window.as_ref().unwrap().set_taskbar_icon(icon);
                }
                OutputEvent::ChangeCursor(cursor) => {
                    if let Some(cursor) = cursor {
                        self.window.as_ref().unwrap().set_cursor(cursor.clone());
                        self.window.as_ref().unwrap().set_cursor_visible(true);
                    } else {
                        self.window.as_ref().unwrap().set_cursor_visible(false);
                    }
                }
            }
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

        let constructor = self.constructor.as_mut();
        let backend = self.backend.as_mut().unwrap();
        let backend_static = unsafe {
            // SAFETY: We are the only thread that can access the backend.
            // and it's guaranteed to pipeline be dropped before the backend.
            mem::transmute::<&mut RendererBackend<E>, &'static mut RendererBackend<E>>(backend)
        };
        self.chain = Some((constructor)(backend_static).unwrap());

        // Notify the monitor about the pass names
        let pass_names = self.chain.as_ref().unwrap().get_names().clone();
        self.monitor.set_pass_names(&pass_names);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Receiver may be dead. We don't care.
        let _ = self.input_out.send(InputEvent(event.clone()));

        match event {
            WindowEvent::Resized(size) => {
                let backend = self.backend.as_mut().unwrap();
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
                let window = self.window.as_ref().unwrap();
                window.pre_present_notify();

                // Render the frame
                self.monitor.render_start();
                if let Err(e) = self.backend.as_mut().unwrap().before_frame() {
                    todo!()
                }

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

                let mut ctx = ChainExecuteCtx::new(frame, self.backend.as_mut().unwrap());

                let pass_result = self.chain.as_mut().unwrap().execute(&mut ctx);
                if let RenderResult::Failed = pass_result {
                    todo!()
                }

                // Do not include after frame in the monitoring, because it usually synchronizes
                // the rendered frame with the OS by swapping buffer, that usually is synchronized
                // with the refresh rate of the display. So this will not be informative.
                self.monitor.render_stop(pass_result, &ctx.durations);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.window.as_ref().unwrap().request_redraw();

        self.monitor.view_stop();

        if let Err(e) = self.backend.as_mut().unwrap().after_frame() {
            todo!()
        }

        // Meet with the Main thread again.
        self.after_frame.wait();
    }
}
