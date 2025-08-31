pub(crate) mod backend;
mod ecs;
mod monitor;

use crate::input::InputEvent;
use crate::passes::chain::RenderChain;
use crate::passes::events::{PassEventTrait, RenderPassEvent};
use crate::passes::pipeline::RenderPipeline;
use crate::passes::result::RenderResult;
use crate::passes::ChainExecuteCtx;
use crate::renderable::Renderable;
use crate::renderer::backend::{RendererBackendError, RendererBackendTrait};
use crate::renderer::ecs::attach_to_ecs;
use crate::renderer::monitor::{DummyRendererMonitor, RendererMonitor, RendererMonitorTrait};
use crate::view::{TickResult, View, ViewConfig, ViewCursor, ViewError, ViewGeometry, ViewTrait};
use crossbeam_channel::{unbounded, Receiver, Sender};
use evenio::component::Component;
use evenio::event::GlobalEvent;
use evenio::world::World;
use log::{info, warn};
use std::panic::UnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use triple_buffer::{triple_buffer, Input, Output};

// Re-export the necessary types for user
pub use backend::{RendererBackend, RendererBackendConfig};
use dawn_util::rendezvous::Rendezvous;
pub use monitor::RendererMonitorEvent;

#[derive(GlobalEvent, Clone)]
pub enum ViewEvent {
    SetGeometry(ViewGeometry),
    SetCursor(ViewCursor),
    SetTitle(String),
}

#[derive(Clone)]
pub(crate) struct DataStreamFrame {
    epoch: usize,
    renderables: Vec<Renderable>,
}

#[derive(Component)]
pub struct Renderer<E: PassEventTrait> {
    stop_signal: Arc<AtomicBool>,
    // Used for streaming renderables to the renderer thread
    // This is a triple buffer, so it can be used to read and write renderables
    // without blocking the renderer thread.
    data_stream: Input<DataStreamFrame>,
    // Used for transferring input events from the renderer thread to the ECS.
    inputs_receiver: Receiver<InputEvent>,
    // Used for transferring view events from the ECS to the View.
    view_sender: Sender<ViewEvent>,
    // Used for transferring render pass events from the ECS to the renderer thread.
    renderer_sender: Sender<RenderPassEvent<E>>,
    monitor_receiver: Receiver<RendererMonitorEvent>,
    handle: Option<JoinHandle<()>>,
}

#[derive(Debug)]
pub enum RendererError {
    ViewCreateError(ViewError),
    RendererThreadSetupFailed,
    BackendCreateError(RendererBackendError),
    PipelineCreateError(String),
    ViewTickError(ViewError),
    BackendRenderError(RendererBackendError),
    PipelineExecuteError(),
    MonitoringSetupFailed,
}

impl std::fmt::Display for RendererError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RendererError::ViewCreateError(e) => write!(f, "Failed to create view: {}", e),
            RendererError::RendererThreadSetupFailed => {
                write!(f, "Failed to setup renderer thread")
            }
            RendererError::BackendCreateError(e) => write!(f, "Failed to create backend: {}", e),
            RendererError::ViewTickError(e) => write!(f, "View tick error: {}", e),
            RendererError::BackendRenderError(e) => write!(f, "Backend tick error: {}", e),
            RendererError::MonitoringSetupFailed => write!(f, "Failed to setup monitor"),
            RendererError::PipelineExecuteError() => write!(f, "Failed to execute render pipeline"),
            RendererError::PipelineCreateError(s) => {
                write!(f, "Failed to create render pipeline: {}", s)
            }
        }
    }
}

impl std::error::Error for RendererError {}

impl<E: PassEventTrait> Drop for Renderer<E> {
    fn drop(&mut self) {
        info!("Stopping renderer thread");

        // Ask the renderer thread to stop
        self.stop_signal.store(true, Ordering::Relaxed);

        // Wait for the renderer thread to finish
        // If the thread is already finished, this will do nothing
        if let Some(handle) = self.handle.take() {
            if let Err(e) = handle.join() {
                warn!("Failed to join renderer thread: {:?}", e);
            }
        }
    }
}

trait RendezvousTrait: Send + Sync + 'static + UnwindSafe {
    fn wait(&self) {}
    fn unlock(&self) {}
}

struct RendezvousWrapper(Rendezvous);
impl RendezvousTrait for RendezvousWrapper {
    fn wait(&self) {
        self.0.wait();
    }
    fn unlock(&self) {
        self.0.unlock();
    }
}

struct DummyRendezvous;
impl RendezvousTrait for DummyRendezvous {}

pub trait RenderChainConstructor<C, E> = FnOnce(&mut RendererBackend<E>) -> Result<RenderPipeline<C, E>, String>
    + Send
    + Sync
    + 'static
    + UnwindSafe
where
    C: RenderChain<E>,
    E: PassEventTrait;

impl<E: PassEventTrait> Renderer<E> {
    /// Creates a new renderer instance that will immediately try to spawn a View,
    /// RendererBackend and all the necessary threads to run the rendering loop.
    ///
    /// `view_config` is the configuration for the View that will be created. The
    /// content of this structure is OS-dependent, so you should wrap it in a
    /// `cfg` attribute.
    ///
    /// `backend_config` is the configuration for the RendererBackend that will be created.
    /// The content of this structure is renderer-backend-dependent, so you should
    /// wrap it in a `cfg` attribute as well. The renderer backend is selected via
    /// feature flags, so you can choose which backend to use.
    ///
    /// `constructor` is a function that will be called to create the render pipeline.
    /// It is called once after the view and backend are created. So you can safely
    /// allocate resources in it.
    pub fn new<C>(
        view_config: ViewConfig,
        backend_config: RendererBackendConfig,
        constructor: impl RenderChainConstructor<C, E>,
    ) -> Result<Self, RendererError>
    where
        C: RenderChain<E>,
    {
        if let Some(sync) = view_config.synchronization.clone() {
            Self::new_inner(
                view_config,
                backend_config,
                constructor,
                RendezvousWrapper(sync.before_frame),
                RendezvousWrapper(sync.after_frame),
                DummyRendererMonitor {},
            )
        } else {
            Self::new_inner(
                view_config,
                backend_config,
                constructor,
                DummyRendezvous {},
                DummyRendezvous {},
                DummyRendererMonitor {},
            )
        }
    }

    /// Creates a new renderer instance with enabled monitoring.
    /// Monitoring is only beneficial if the renderer is attached to the ECS -
    /// it will send monitoring data to the ECS every second.
    /// That may affect the performance of the renderer.
    ///
    /// See more information on the function `new`.
    pub fn new_with_monitoring<C>(
        view_config: ViewConfig,
        backend_config: RendererBackendConfig,
        constructor: impl RenderChainConstructor<C, E>,
    ) -> Result<Self, RendererError>
    where
        C: RenderChain<E>,
    {
        if let Some(sync) = view_config.synchronization.clone() {
            Self::new_inner(
                view_config,
                backend_config,
                constructor,
                RendezvousWrapper(sync.before_frame),
                RendezvousWrapper(sync.after_frame),
                RendererMonitor::new(),
            )
        } else {
            Self::new_inner(
                view_config,
                backend_config,
                constructor,
                DummyRendezvous {},
                DummyRendezvous {},
                RendererMonitor::new(),
            )
        }
    }

    fn new_inner<P, C>(
        view_config: ViewConfig,
        backend_config: RendererBackendConfig,
        constructor: impl RenderChainConstructor<C, E>,
        before_frame: impl RendezvousTrait,
        after_frame: impl RendezvousTrait,
        mut monitor: P,
    ) -> Result<Self, RendererError>
    where
        P: RendererMonitorTrait,
        C: RenderChain<E>,
    {
        // Setup monitor
        let (monitor_sender, monitor_receiver) = unbounded();
        monitor.set_sender(monitor_sender.clone());

        // Setup renderer
        let (inputs_sender, inputs_receiver) = unbounded();
        let (view_sender, view_receiver) = unbounded();
        let (renderer_sender, renderer_receiver) = unbounded();
        let (stream_input, mut stream_output) =
            triple_buffer::<DataStreamFrame>(&DataStreamFrame {
                epoch: 0,
                renderables: vec![],
            });
        let stop_signal = Arc::new(AtomicBool::new(false));

        let stop_signal_clone = stop_signal.clone();
        let handle = Builder::new()
            .name("renderer".to_string())
            .spawn(move || {
                info!("Renderer thread started");

                let func = || {
                    // Create the view, backend and the rendering pipeline
                    let mut view = View::open(view_config, inputs_sender)
                        .map_err(RendererError::ViewCreateError)?;
                    let mut backend = RendererBackend::<E>::new(backend_config, view.get_handle())
                        .map_err(RendererError::BackendCreateError)?;
                    let mut pipeline =
                        constructor(&mut backend).map_err(RendererError::PipelineCreateError)?;

                    // Notify the monitor about the pass names
                    let pass_names = pipeline.get_names();
                    monitor.set_pass_names(&pass_names);

                    info!("Starting renderer loop");
                    let mut frame_index = 0;
                    while !stop_signal_clone.load(Ordering::SeqCst) {
                        // This has no sense if no synchronization is disabled,
                        // but if it is, it is a good idea to process all the events between frames.
                        // Here, in the meanwhile, the Main thread will copy the renderables to us,
                        // so we have some time to handle events.
                        // It also guarantees that all the events the user produced will be processed
                        // before the next frame.
                        Self::handle_events(&mut monitor, &mut pipeline, &renderer_receiver)?;

                        // Meet with the Main thread
                        before_frame.wait();

                        // Get the new events from the OS
                        match Self::handle_view(&mut view, &mut monitor, &view_receiver) {
                            Ok(false) => {
                                before_frame.unlock();
                                after_frame.unlock();
                                return Ok(());
                            }
                            Err(e) => {
                                Err(e)?;
                            }
                            _ => {}
                        }

                        // Render the frame
                        frame_index = Self::handle_render(
                            frame_index,
                            &mut monitor,
                            &mut backend,
                            &mut stream_output,
                            &mut pipeline,
                        )?;

                        // Meet with the Main thread again.
                        after_frame.wait();
                    }

                    Ok(())
                };

                // TODO: Handle panics in the renderer thread
                let err: Result<(), RendererError> = func();

                // Request other threads to stop
                stop_signal_clone.store(true, Ordering::SeqCst);
                info!("Renderer thread finished");

                if let Err(e) = err {
                    warn!("Renderer thread error: {:?}", e);
                }
            })
            .map_err(|_| RendererError::RendererThreadSetupFailed)?;

        Ok(Self {
            stop_signal,
            data_stream: stream_input,
            inputs_receiver,
            view_sender,
            renderer_sender,
            monitor_receiver,
            handle: Some(handle),
        })
    }

    #[inline(always)]
    fn handle_view(
        view: &mut View,
        monitor: &mut impl RendererMonitorTrait,
        view_receiver: &Receiver<ViewEvent>,
    ) -> Result<bool, RendererError> {
        // Process View. Usually this will produce input events
        monitor.view_start();
        for event in view_receiver.try_iter() {
            match event {
                ViewEvent::SetGeometry(geo) => {
                    if let Err(e) = view.set_geometry(geo) {
                        warn!("Failed to set view geometry: {:?}", e);
                    }
                }
                ViewEvent::SetCursor(cursor) => {
                    if let Err(e) = view.set_cursor(cursor) {
                        warn!("Failed to set view cursor: {:?}", e);
                    }
                }
                ViewEvent::SetTitle(title) => {
                    if let Err(e) = view.set_title(&title) {
                        warn!("Failed to set view title: {:?}", e);
                    }
                }
            }
        }

        match view.tick() {
            TickResult::Continue => {
                // View tick was successful, continue processing
                return Ok(true);
            }
            TickResult::Closed => {
                // View tick returned false, which means the view was closed
                info!("View closed, stopping renderer thread");
                return Ok(false);
            }
            TickResult::Failed(e) => {
                // An error occurred during the view tick
                warn!("View tick error: {:?}", e);
                Err(RendererError::ViewTickError(e))?;
            }
        }
        monitor.view_stop();
        Ok(true)
    }

    #[inline(always)]
    fn handle_events<C>(
        monitor: &mut impl RendererMonitorTrait,
        pipeline: &mut RenderPipeline<C, E>,
        renderer_queue: &Receiver<RenderPassEvent<E>>,
    ) -> Result<(), RendererError>
    where
        C: RenderChain<E>,
    {
        monitor.events_start();
        for event in renderer_queue.try_iter() {
            pipeline.dispatch(event);
        }
        monitor.events_stop();

        Ok(())
    }

    #[inline(always)]
    fn handle_render<C>(
        mut frame_index: usize,
        monitor: &mut impl RendererMonitorTrait,
        backend: &mut RendererBackend<E>,
        stream: &mut Output<DataStreamFrame>,
        pipeline: &mut RenderPipeline<C, E>,
    ) -> Result<usize, RendererError>
    where
        C: RenderChain<E>,
    {
        monitor.render_start();
        if let Err(e) = backend.before_frame() {
            return Err(RendererError::BackendRenderError(e));
        }

        let frame = stream.read();
        if frame.epoch != frame_index {
            warn!(
                "Renderer is out of sync! Expected epoch {}, got {}",
                frame_index, frame.epoch
            );
            frame_index = frame.epoch;
        } else {
            frame_index += 1;
        }

        let mut ctx = ChainExecuteCtx::new(frame.renderables.as_slice(), backend);

        let pass_result = pipeline.execute(&mut ctx);
        if let RenderResult::Failed = pass_result {
            return Err(RendererError::PipelineExecuteError());
        }

        // Do not include after frame in the monitoring, because it usually synchronizes
        // the rendered frame with the OS by swapping buffer, that usually is synchronized
        // with the refresh rate of the display. So this will not be informative.
        monitor.render_stop(pass_result, &ctx.durations);

        if let Err(e) = backend.after_frame() {
            return Err(RendererError::BackendRenderError(e))?;
        }

        Ok(frame_index)
    }

    /// After attaching the renderer to the ECS, it will automatically collect the renderables
    /// and send them to the renderer thread (see `renderable` mod for more details).
    ///
    /// Input events from the ECS:
    ///    - `ViewEvent` - to control the view (resize, set cursor, set title)
    ///    - `RenderPassEvent<E>` - to send events to the render passes
    ///
    /// Output events to the ECS:
    ///   - `InputEvent` - when any input event is received from the OS
    ///   - `RendererMonitorEvent` - when monitoring is enabled, it will send monitoring data every second
    ///   - `ExitEvent` - when the view is closed or the renderer fails, it will send an exit event to stop the ECS
    ///
    /// This function moves the renderer into the ECS world.
    pub fn attach_to_ecs(self, world: &mut World) {
        attach_to_ecs::<E>(self, world);
    }
}
