mod app;
pub(crate) mod backend;
mod ecs;
mod monitor;

use crate::passes::chain::RenderChain;
use crate::passes::events::{PassEventTrait, RenderPassEvent};
use crate::passes::pipeline::RenderPipeline;
use crate::renderable::{
    Renderable, RenderableAreaLight, RenderablePointLight, RenderableSpotLight, RenderableSunLight,
};
use crate::renderer::app::Application;
use crate::renderer::backend::RendererBackendError;
use crate::renderer::ecs::attach_to_ecs;
use crate::renderer::monitor::{DummyRendererMonitor, RendererMonitor, RendererMonitorTrait};
pub use backend::{RendererBackend, RendererConfig};
use crossbeam_channel::{unbounded, Receiver, Sender};
use dawn_util::rendezvous::Rendezvous;
use evenio::component::Component;
use evenio::event::GlobalEvent;
use evenio::world::World;
use glam::UVec2;
use log::info;
pub use monitor::RendererMonitorEvent;
use std::panic::UnwindSafe;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use triple_buffer::{triple_buffer, Input};
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::{Cursor, Icon};

#[derive(Clone)]
pub struct WindowConfig {
    /// Allows to enable additional synchronization between threads.
    /// For example, to synchronize rendering and logic threads.
    pub synchronization: Option<RendererSynchronization>,

    /// Initial title of the window.
    pub title: String,
    /// Initial dimensions of the window.
    pub dimensions: UVec2,
    /// Whether the window is resizable by the user.
    pub resizable: bool,
    /// Whether the window has decorations (title bar, borders, etc.).
    pub decorations: bool,
    /// Whether the window should start in fullscreen mode.
    pub fullscreen: bool,
    /// Initial position of the window.
    pub icon: Option<Icon>,
    /// Initial cursor of the window. None means hidden cursor.
    pub cursor: Option<Cursor>,
}

#[derive(Clone)]
pub struct RendererSynchronization {
    pub before_frame: Rendezvous,
    pub after_frame: Rendezvous,
}

/// Input events from the Window/OS to the ECS.
/// For example, keyboard and mouse events, window resize, etc.
/// See `winit::event::WindowEvent` for more details.
/// Forced to wrap in a struct to implement GlobalEvent.
#[derive(GlobalEvent, Clone)]
#[repr(transparent)]
pub struct InputEvent(pub WindowEvent);

/// Output events from the ECS to the Window/OS.
/// For example, window resize, set cursor, set title, etc.
#[derive(GlobalEvent, Clone)]
pub enum OutputEvent {
    ChangeTitle(String),
    ChangeWindowSize(UVec2),
    ChangeResizable(bool),
    ChangeDecorations(bool),
    ChangeFullscreen(bool),
    ChangeIcon(Option<Icon>),
    ChangeCursor(Option<Cursor>),
}

/// Frame data that is streamed to the renderer thread.
/// It contains all the renderables and lights to be rendered in the current frame.
/// This structure is sent via a triple buffer, so it can be updated
/// in the ECS thread without blocking the renderer thread.
#[derive(Clone)]
pub struct DataStreamFrame {
    /// Incremented every time the frame is updated.
    pub epoch: usize,
    /// All renderables to be rendered in the current frame.
    pub renderables: Vec<Renderable>,
    /// All point lights to be rendered in the current frame.
    pub point_lights: Vec<RenderablePointLight>,
    /// All spot lights to be rendered in the current frame.
    pub spot_lights: Vec<RenderableSpotLight>,
    /// All area lights to be rendered in the current frame.
    pub area_lights: Vec<RenderableAreaLight>,
    /// All sun lights to be rendered in the current frame.
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

pub struct Renderer {
    run: Box<dyn FnOnce() + Send + 'static>,
}

/// Thread-shared proxy to communicate with the renderer thread.
/// It contains channels and triple buffers to send and receive data.
/// After attaching the renderer to the ECS, it will automatically
/// collect the renderables and send them to the renderer thread.
/// See `renderable` mod for more details.
#[derive(Component)]
pub struct RendererProxy<E: PassEventTrait> {
    stop_signal: Arc<AtomicBool>,
    // Used for streaming renderables to the renderer thread
    // This is a triple buffer, so it can be used to read and write renderables
    // without blocking the renderer thread.
    data_stream: Input<DataStreamFrame>,
    // Used for transferring input events from the renderer thread to the ECS.
    input_receiver: Receiver<InputEvent>,
    // Used for transferring events to the Window from the ECS to the renderer thread.
    output_sender: Sender<OutputEvent>,
    // Used for transferring render pass events from the ECS to the renderer thread.
    renderer_sender: Sender<RenderPassEvent<E>>,
    monitor_receiver: Receiver<RendererMonitorEvent>,
}

impl<E: PassEventTrait> Drop for RendererProxy<E> {
    fn drop(&mut self) {
        info!("RendererProxy dropped, stopping renderer");
        self.stop_signal
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
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

pub trait RenderChainConstructor<C, E> = Fn(&'static mut RendererBackend<E>) -> Result<RenderPipeline<C, E>, String>
    + Send
    + Sync
    + 'static
where
    C: RenderChain<E>,
    E: PassEventTrait;

impl Renderer {
    /// Creates a new renderer instance and its proxy.
    /// The renderer instance should be run in a main thread using the `run` method.
    ///
    /// Usually you want to attach the renderer proxy to the ECS using the `attach_to_ecs` method
    /// to control it from the ECS.
    ///
    /// If you want to enable synchronization between the renderer and logic threads,
    /// you can provide a `RendererSynchronization` in the `WindowConfig`.
    /// This will allow you to synchronize the threads using the provided `Rendezvous`.
    ///
    /// Monitoring is disabled by default - if you want to enable it, use the `new_with_monitoring` method.
    /// Monitoring is only beneficial if the renderer is attached to the ECS -
    /// it will eventually send monitoring data to the ECS.
    /// That may affect the performance of the renderer.
    pub fn new<C, E>(
        window_config: WindowConfig,
        backend_config: RendererConfig,
        constructor: impl RenderChainConstructor<C, E>,
    ) -> Result<(Renderer, RendererProxy<E>), RendererError>
    where
        E: PassEventTrait,
        C: RenderChain<E>,
    {
        if let Some(sync) = window_config.synchronization.clone() {
            Self::new_inner(
                window_config,
                backend_config,
                constructor,
                RendezvousWrapper(sync.before_frame),
                RendezvousWrapper(sync.after_frame),
                DummyRendererMonitor {},
            )
        } else {
            Self::new_inner(
                window_config,
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
    /// it will eventually send monitoring data to the ECS.
    /// That may affect the performance of the renderer.
    ///
    /// See more information on the function `new`.
    pub fn new_with_monitoring<C, E>(
        window_config: WindowConfig,
        backend_config: RendererConfig,
        constructor: impl RenderChainConstructor<C, E>,
    ) -> Result<(Renderer, RendererProxy<E>), RendererError>
    where
        E: PassEventTrait,
        C: RenderChain<E>,
    {
        if let Some(sync) = window_config.synchronization.clone() {
            Self::new_inner(
                window_config,
                backend_config,
                constructor,
                RendezvousWrapper(sync.before_frame),
                RendezvousWrapper(sync.after_frame),
                RendererMonitor::new(),
            )
        } else {
            Self::new_inner(
                window_config,
                backend_config,
                constructor,
                DummyRendezvous {},
                DummyRendezvous {},
                RendererMonitor::new(),
            )
        }
    }

    fn new_inner<P, C, E>(
        window_config: WindowConfig,
        backend_config: RendererConfig,
        constructor: impl RenderChainConstructor<C, E>,
        before_frame: impl RendezvousTrait,
        after_frame: impl RendezvousTrait,
        mut monitor: P,
    ) -> Result<(Renderer, RendererProxy<E>), RendererError>
    where
        E: PassEventTrait,
        P: RendererMonitorTrait,
        C: RenderChain<E>,
    {
        // Setup monitor
        let (monitor_sender, monitor_receiver) = unbounded();
        monitor.set_sender(monitor_sender.clone());

        // Setup renderer
        let (input_sender, input_receiver) = unbounded();
        let (output_sender, output_receiver) = unbounded();
        let (renderer_sender, renderer_receiver) = unbounded();
        let (stream_input, stream_output) = triple_buffer::<DataStreamFrame>(&DataStreamFrame {
            epoch: 0,
            renderables: vec![],
            point_lights: vec![],
            spot_lights: vec![],
            area_lights: vec![],
            sun_lights: vec![],
        });
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = Arc::clone(&stop_signal);

        Ok((
            Renderer {
                run: Box::new(move || {
                    let mut app = Application::new(
                        window_config,
                        backend_config,
                        monitor,
                        constructor,
                        before_frame,
                        after_frame,
                        stop_signal.clone(),
                        renderer_receiver,
                        output_receiver,
                        stream_output,
                        input_sender,
                    )
                    .unwrap();

                    info!("Starting renderer event loop");
                    let event_loop = EventLoop::new().unwrap();
                    event_loop.run_app(&mut app).unwrap();
                    info!("Renderer event loop has exited");
                }),
            },
            RendererProxy::<E> {
                stop_signal: stop_signal_clone,
                data_stream: stream_input,
                input_receiver,
                output_sender,
                renderer_sender,
                monitor_receiver,
            },
        ))
    }

    /// Consumes the renderer and runs it in the current thread.
    /// This function will block the current thread until the renderer
    /// exits (for example, when the window is closed).
    pub fn run(self) {
        (self.run)()
    }
}

impl<E: PassEventTrait> RendererProxy<E> {
    /// After attaching the renderer proxy to the ECS, it will automatically collect the
    /// information about renderables and lights from the ECS every
    /// frame and send it to the renderer thread.
    ///
    /// Input events from the ECS:
    ///    - `OutputEvent` - to control the Window (resize, set cursor, set title)
    ///    - `RenderPassEvent<E>` - to send events to the render passes
    ///
    /// Output events to the ECS:
    ///   - `InputEvent` - when any input event is received from the OS
    ///   - `RendererMonitorEvent` - when monitoring is enabled, it will send monitoring data every second
    ///   - `ExitEvent` - when the Window is closed or the renderer fails, it will send an exit event to stop the ECS
    ///
    /// This function moves the renderer into the ECS world.
    pub fn attach_to_ecs(self, world: &mut World) {
        attach_to_ecs::<E>(self, world);
    }
}
