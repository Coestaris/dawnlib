pub(crate) mod backend;
mod cycle;
mod ecs;
mod monitor;

use crate::passes::chain::RenderChain;
use crate::passes::events::{PassEventTrait, RenderPassEvent};
use crate::passes::pipeline::RenderPipeline;
use crate::renderable::{
    Renderable, RenderableAreaLight, RenderablePointLight, RenderableSpotLight, RenderableSunLight,
};
use crate::renderer::backend::RendererBackendError;
use crate::renderer::cycle::Cycle;
use crate::renderer::ecs::attach_to_ecs;
use crate::renderer::monitor::{DummyRendererMonitor, RendererMonitor, RendererMonitorTrait};
pub use backend::{RendererBackend, RendererBackendConfig};
use crossbeam_channel::{unbounded, Receiver, Sender};
use dawn_util::rendezvous::Rendezvous;
use evenio::component::Component;
use evenio::event::GlobalEvent;
use evenio::world::World;
use log::{debug, info, warn};
pub use monitor::RendererMonitorEvent;
use std::panic::UnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use triple_buffer::{triple_buffer, Input};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;

#[derive(Clone)]
pub struct ViewConfig {
    /// Allows to enable additional synchronization between threads.
    /// For example, to synchronize rendering and logic threads.
    pub synchronization: Option<ViewSynchronization>,

    /// Title of the window
    pub title: String,
}

#[derive(Clone)]
pub struct ViewSynchronization {
    pub before_frame: Rendezvous,
    pub after_frame: Rendezvous,
}

#[derive(GlobalEvent, Clone)]
#[repr(transparent)]
pub struct InputEvent(pub WindowEvent);

#[derive(GlobalEvent, Clone)]
pub enum ViewEvent {}

#[derive(Clone)]
pub struct DataStreamFrame {
    pub epoch: usize,
    pub renderables: Vec<Renderable>,

    pub point_lights: Vec<RenderablePointLight>,
    pub spot_lights: Vec<RenderableSpotLight>,
    pub area_lights: Vec<RenderableAreaLight>,
    pub sun_lights: Vec<RenderableSunLight>,
}

impl DataStreamFrame {
    pub fn clear(&mut self) {
        self.renderables.clear();
        self.point_lights.clear();
        self.spot_lights.clear();
        self.area_lights.clear();
        self.sun_lights.clear();
    }
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
    RendererThreadSetupFailed,
    BackendCreateError(RendererBackendError),
    PipelineCreateError(String),
    BackendRenderError(RendererBackendError),
    PipelineExecuteError(),
    MonitoringSetupFailed,
}

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

pub trait RenderChainConstructor<C, E> =
    Fn(&mut RendererBackend<E>) -> Result<RenderPipeline<C, E>, String> + Send + Sync + 'static
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
                point_lights: vec![],
                spot_lights: vec![],
                area_lights: vec![],
                sun_lights: vec![],
            });
        let stop_signal = Arc::new(AtomicBool::new(false));

        let stop_signal_clone_1 = stop_signal.clone();
        let stop_signal_clone_2 = stop_signal.clone();
        let handle = Builder::new()
            .name("renderer".to_string())
            .spawn(move || {
                info!("Renderer thread started");

                let mut cycle = Cycle::new(
                    // Here we go
                    view_config,
                    backend_config,
                    monitor,
                    constructor,
                    before_frame,
                    after_frame,
                    stop_signal_clone_1,
                    renderer_receiver,
                    view_receiver,
                    stream_output,
                    inputs_sender,
                )
                .unwrap();

                let event_loop = EventLoop::new();

                info!("Starting event loop");
                event_loop.unwrap().run_app(&mut cycle).unwrap();

                // Request other threads to stop
                stop_signal_clone_2.store(true, Ordering::SeqCst);

                info!("Renderer thread finished");
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
