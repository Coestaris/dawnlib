use crate::input::InputEvent;
use crate::renderable::{Material, Position, Renderable, RenderableMesh, Rotation, Scale};
use crate::view::{TickResult, View, ViewConfig, ViewError, ViewHandle, ViewTrait};
use crossbeam_queue::ArrayQueue;
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver, Sender};
use evenio::fetch::{Fetcher, Single};
use evenio::query::Query;
use evenio::world::World;
use glam::Vec3;
use log::{debug, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use triple_buffer::{triple_buffer, Input, Output};
use yage2_core::ecs::{StopEventLoop, Tick};
use yage2_core::profile::{PeriodProfiler, ProfileFrame, TickProfiler};

pub(crate) struct RendererTickResult {
    pub draw_calls: usize,
    pub drawn_primitives: usize,
}

pub(crate) trait RendererBackendTrait {
    fn new(
        config: RendererBackendConfig,
        view_handle: ViewHandle,
    ) -> Result<Self, RendererBackendError>
    where
        Self: Sized;

    fn tick(
        &mut self,
        renderables: &[Renderable],
    ) -> Result<RendererTickResult, RendererBackendError>;
}

#[cfg(feature = "gl")]
mod backend_impl {
    pub type RendererBackend = crate::gl::GLRenderer;
    pub type RendererBackendConfig = crate::gl::GLRendererConfig;
    pub type RendererBackendError = crate::gl::GLRendererError;
}

pub use backend_impl::*;

const STATISTICS_THREAD_NAME: &str = "ren_stats";
const INPUTS_QUEUE_CAPACITY: usize = 1024;
const PROFILE_QUEUE_CAPACITY: usize = 32;

#[derive(Component)]
pub struct Renderer {
    stop_signal: Arc<AtomicBool>,
    renderables_buffer_input: Input<Vec<Renderable>>,
    inputs_queue: Arc<ArrayQueue<InputEvent>>,
    profile_frames: Arc<ArrayQueue<RendererProfileFrame>>,
    handle: Option<JoinHandle<()>>,
}

#[derive(GlobalEvent)]
pub struct RendererProfileFrame {
    pub fps: ProfileFrame,
    pub view_tick: ProfileFrame,
    pub backend_tick: ProfileFrame,
    pub drawn_primitives: ProfileFrame,
    pub draw_calls: ProfileFrame,
}

trait RendererProfilerTrait {
    fn view_tick_start(&self) {}
    fn view_tick_end(&self) {}
    fn backend_tick_start(&self) {}
    fn backend_tick_end(&self) {}
    fn draw_result(&self, _tick_result: RendererTickResult) {}
    fn spawn_thread(
        self: Arc<Self>,
        _stop_signal: Arc<AtomicBool>,
        _sender: Arc<ArrayQueue<RendererProfileFrame>>,
    ) -> Result<(), RendererError> {
        Ok(())
    }
}

struct RendererProfiler {
    fps: TickProfiler,
    view_tick: PeriodProfiler,
    backend_tick: PeriodProfiler,
    draw_calls: TickProfiler,
    drawn_primitives: TickProfiler,
    handle: Option<JoinHandle<()>>,
}

impl RendererProfilerTrait for RendererProfiler {
    fn view_tick_start(&self) {
        self.fps.tick(1);
        self.view_tick.start();
    }

    fn view_tick_end(&self) {
        self.view_tick.end();
    }

    fn backend_tick_start(&self) {
        self.backend_tick.start();
    }

    fn backend_tick_end(&self) {
        self.backend_tick.end();
    }

    fn draw_result(&self, tick_result: RendererTickResult) {
        self.drawn_primitives
            .tick(tick_result.drawn_primitives as u32);
        self.draw_calls.tick(tick_result.draw_calls as u32);
    }

    fn spawn_thread(
        self: Arc<Self>,
        stop_signal: Arc<AtomicBool>,
        sender: Arc<ArrayQueue<RendererProfileFrame>>,
    ) -> Result<(), RendererError> {
        Builder::new()
            .name(STATISTICS_THREAD_NAME.to_string())
            .spawn(move || {
                loop {
                    // Check if the stop signal is set
                    if stop_signal.load(Ordering::Relaxed) {
                        debug!("Received stop signal");
                        break;
                    }

                    self.fps.update();
                    self.drawn_primitives.update();

                    let frame = RendererProfileFrame {
                        fps: self.fps.get_frame(),
                        view_tick: self.view_tick.get_frame(),
                        backend_tick: self.backend_tick.get_frame(),
                        drawn_primitives: self.drawn_primitives.get_frame(),
                        draw_calls: self.draw_calls.get_frame(),
                    };

                    let _ = sender.push(frame);

                    std::thread::sleep(std::time::Duration::from_secs(1));
                }

                info!("Renderer profiler thread finished");
            })
            .map_err(|_| RendererError::RendererThreadSetupFailed)?;
        Ok(())
    }
}

impl RendererProfiler {
    pub fn new() -> Self {
        RendererProfiler {
            fps: TickProfiler::new(0.5),
            view_tick: PeriodProfiler::new(0.5),
            backend_tick: PeriodProfiler::new(0.5),
            drawn_primitives: TickProfiler::new(1.0),
            draw_calls: TickProfiler::new(1.0),
            handle: None,
        }
    }
}

impl Drop for RendererProfiler {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            if let Err(e) = handle.join() {
                warn!("Failed to join renderer profiler thread: {:?}", e);
            }
        }
    }
}

struct DummyRendererProfiler {}

impl RendererProfilerTrait for DummyRendererProfiler {}

#[derive(Debug)]
pub enum RendererError {
    ViewCreateError(ViewError),
    RendererThreadSetupFailed,
    BackendCreateError(RendererBackendError),
    ViewTickError(ViewError),
    BackendTickError(RendererBackendError),
    ProfilerSetupFailed,
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
            RendererError::BackendTickError(e) => write!(f, "Backend tick error: {}", e),
            RendererError::ProfilerSetupFailed => write!(f, "Failed to setup profiler"),
        }
    }
}

impl std::error::Error for RendererError {}

impl Drop for Renderer {
    fn drop(&mut self) {
        info!("Stopping renderer thread");
        self.stop_signal.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            if let Err(e) = handle.join() {
                warn!("Failed to join renderer thread: {:?}", e);
            }
        }
    }
}

impl Renderer {
    pub fn new(
        view_config: ViewConfig,
        backend_config: RendererBackendConfig,
        use_profiling: bool,
    ) -> Result<Self, RendererError> {
        if use_profiling {
            Self::new_inner(view_config, backend_config, RendererProfiler::new())
        } else {
            Self::new_inner(view_config, backend_config, DummyRendererProfiler {})
        }
    }

    fn new_inner<P>(
        view_config: ViewConfig,
        backend_config: RendererBackendConfig,
        profiler: P,
    ) -> Result<Self, RendererError>
    where
        P: RendererProfilerTrait + Send + Sync + 'static,
    {
        // Setup profiler
        let stop_signal = Arc::new(AtomicBool::new(false));
        let profiler = Arc::new(profiler);
        let profile_frames = Arc::new(ArrayQueue::<RendererProfileFrame>::new(
            PROFILE_QUEUE_CAPACITY,
        ));
        profiler
            .clone()
            .spawn_thread(stop_signal.clone(), profile_frames.clone())
            .map_err(|_| RendererError::ProfilerSetupFailed)?;

        // Setup renderer
        let inputs_queue = Arc::new(ArrayQueue::<InputEvent>::new(INPUTS_QUEUE_CAPACITY));
        let inputs_queue_clone = inputs_queue.clone();
        let stop_signal_clone1 = stop_signal.clone();
        let stop_signal_clone2 = stop_signal.clone();
        let initial = vec![];
        let (renderables_buffer_input, renderables_buffer_output) =
            triple_buffer::<Vec<Renderable>>(&initial);
        let handle = Builder::new()
            .name("renderer".to_string())
            .spawn(move || {
                let err = Self::renderer(
                    view_config,
                    backend_config,
                    inputs_queue_clone,
                    renderables_buffer_output,
                    profiler,
                    stop_signal_clone1,
                );

                // Request other threads to stop
                stop_signal_clone2.store(true, Ordering::SeqCst);
                info!("Renderer thread finished");

                if let Err(e) = err {
                    warn!("Renderer thread error: {:?}", e);
                }
            })
            .map_err(|_| RendererError::RendererThreadSetupFailed)?;

        Ok(Self {
            stop_signal,
            renderables_buffer_input,
            inputs_queue,
            profile_frames,
            handle: Some(handle),
        })
    }

    fn renderer<P>(
        view_config: ViewConfig,
        backend_config: RendererBackendConfig,
        inputs_sender: Arc<ArrayQueue<InputEvent>>,
        mut renderables_buffer: Output<Vec<Renderable>>,
        profiler: Arc<P>,
        stop_signal: Arc<AtomicBool>,
    ) -> Result<(), RendererError>
    where
        P: RendererProfilerTrait + Send + Sync + 'static,
    {
        let mut view =
            View::open(view_config, inputs_sender).map_err(RendererError::ViewCreateError)?;

        let mut backend = RendererBackend::new(backend_config, view.get_handle())
            .map_err(RendererError::BackendCreateError)?;

        info!("Renderer thread started");
        let mut result = Ok(());
        while !stop_signal.load(Ordering::SeqCst) {
            profiler.view_tick_start();
            match view.tick() {
                TickResult::Continue => {
                    // View tick was successful, continue processing
                }
                TickResult::Closed => {
                    // View tick returned false, which means the view was closed
                    info!("View closed, stopping renderer thread");
                    break;
                }
                TickResult::Failed(e) => {
                    // An error occurred during the view tick
                    warn!("View tick error: {:?}", e);
                    result = Err(RendererError::ViewTickError(e));
                    break;
                }
            }
            profiler.view_tick_end();

            profiler.backend_tick_start();
            let renderables = renderables_buffer.read();
            match backend.tick(renderables.as_slice()) {
                Ok(result) => {
                    // Rendering was successful, update the profiler
                    profiler.draw_result(result);
                }
                Err(e) => {
                    // An error occurred during rendering
                    warn!("Backend tick error: {:?}", e);
                    result = Err(RendererError::BackendTickError(e));
                    break;
                }
            }
            profiler.backend_tick_end();
        }

        stop_signal.store(true, Ordering::SeqCst);
        result
    }

    /// After attaching the player to the ECS, it will automatically collect the renderables
    /// and send them to the renderer thread (see `renderable` mod for more details).
    ///
    /// When any input event is received, it will be sent to the ECS as `InputEvent` events.
    ///
    /// Also, if you enabled profiling, it will send profiling data as `RendererProfileFrame`
    /// events to the ECS every second.
    ///
    /// Additionally, if the Window or Renderer is closed/failed the event loop will be stopped
    /// by sending a `StopEventLoop` event to the ECS.
    ///
    /// This function moves the player into the ECS world.
    pub fn attach_to_ecs(self, world: &mut World) {
        // Setup the renderer player entity in the ECS
        let renderer_entity = world.spawn();
        world.insert(renderer_entity, self);

        // If renderer loop is closed or stopped,
        // we need to stop the event loop
        fn view_closed_handler(
            _: Receiver<Tick>,
            renderer: Single<&Renderer>,
            mut sender: Sender<StopEventLoop>,
        ) {
            // Check if the view was closed, if so, send a global event to stop the event loop
            if renderer.0.stop_signal.load(Ordering::Relaxed) {
                info!("View closed, stopping the event loop");
                sender.send(StopEventLoop);
            }
        }

        fn profile_handler(
            _: Receiver<Tick>,
            renderer: Single<&Renderer>,
            mut sender: Sender<RendererProfileFrame>,
        ) {
            // Check if there's any profile frame to process.
            // If so, push them to the ECS
            while let Some(frame) = renderer.0.profile_frames.pop() {
                sender.send(frame);
            }
        }

        fn inputs_handler(
            _: Receiver<Tick>,
            renderer: Single<&Renderer>,
            mut sender: Sender<InputEvent>,
        ) {
            // Check if there's any input event to process.
            // If so, push them to the ECS
            while let Some(input) = renderer.0.inputs_queue.pop() {
                sender.send(input);
            }
        }

        #[derive(Query)]
        struct Query<'a> {
            mesh: &'a RenderableMesh,
            position: Option<&'a Position>,
            rotation: Option<&'a Rotation>,
            scale: Option<&'a Scale>,
            material: Option<&'a Material>,
        }

        fn collect_renderables(
            _: Receiver<Tick>,
            renderer: Single<&mut Renderer>,
            fetcher: Fetcher<Query>,
        ) {
            // TODO: Do not allocate a new vector every time, instead use a static one!
            let mut renderables = Vec::new();
            for query in fetcher.iter() {
                // Collect the renderable data from the query
                let mesh_id = query.mesh.mesh_id;
                let position = query.position.map_or(Vec3::ZERO, |p| p.0);
                let rotation = query.rotation.map_or(Vec3::ZERO, |r| r.0);
                let scale = query.scale.map_or(Vec3::ONE, |s| s.0);
                let material = query.material.map_or(Material::default(), |m| m.clone());

                // Create a new Renderable instance
                let renderable = Renderable {
                    model: glam::Mat4::from_scale_rotation_translation(
                        scale,
                        glam::Quat::from_euler(
                            glam::EulerRot::XYZ,
                            rotation.x,
                            rotation.y,
                            rotation.z,
                        ),
                        position,
                    ),
                    mesh_id,
                    material,
                };

                // Push the renderable to the vector
                renderables.push(renderable);
            }

            // Send the collected renderables to the renderer thread
            renderer.0.renderables_buffer_input.write(renderables);
        }

        // Add handlers to the ECS to process inputs and profiling
        world.add_handler(profile_handler);
        world.add_handler(inputs_handler);
        world.add_handler(view_closed_handler);
        // Add a handler to collect renderables from the ECS
        world.add_handler(collect_renderables);
    }
}
