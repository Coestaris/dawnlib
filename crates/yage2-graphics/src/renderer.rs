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
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use triple_buffer::{triple_buffer, Input, Output};
use yage2_core::ecs::{StopEventLoop, Tick};
use yage2_core::profile::{PeriodProfiler, ProfileFrame, TickProfiler};

pub(crate) trait RendererBackendTrait<C, E>
where
    E: Copy + 'static,
    C: ChainExecute<E> + Send + Sync + 'static,
    Self: Sized,
{
    fn new(
        config: RendererBackendConfig<C, E>,
        view_handle: ViewHandle,
    ) -> Result<Self, RendererBackendError>;

    fn dispatch_event(&mut self, event: &RenderPassEvent<E>) -> Result<(), RendererBackendError>;

    fn render(
        &mut self,
        renderables: &[Renderable],
    ) -> Result<PassExecuteResult, RendererBackendError>;
}

#[cfg(feature = "gl")]
mod backend_impl {
    pub type RendererBackend<C, E> = crate::gl::GLRenderer<C, E>;
    pub type RendererBackendConfig<C, E> = crate::gl::GLRendererConfig<C, E>;
    pub type RendererBackendError = crate::gl::GLRendererError;
}

use crate::passes::chain::ChainExecute;
use crate::passes::events::RenderPassEvent;
use crate::passes::result::PassExecuteResult;
pub use backend_impl::*;

const STATISTICS_THREAD_NAME: &str = "ren_stats";
const INPUTS_QUEUE_CAPACITY: usize = 1024;
const RENDERER_QUEUE_CAPACITY: usize = 1024;
const PROFILE_QUEUE_CAPACITY: usize = 32;

#[derive(Component)]
pub struct Renderer<E>
where
    E: 'static + Copy,
{
    stop_signal: Arc<AtomicBool>,
    // Used for transferring renderables to the renderer thread
    // This is a triple buffer, so it can be used to read and write renderables
    // without blocking the renderer thread.
    renderables_buffer_input: Input<Vec<Renderable>>,
    // Used for transferring input events from the renderer thread to the ECS.
    inputs_queue: Arc<ArrayQueue<InputEvent>>,
    // Used for transferring render pass events from the ECS to the renderer thread.
    renderer_queue: Arc<ArrayQueue<RenderPassEvent<E>>>,
    profile_frames: Arc<ArrayQueue<RendererProfileFrame>>,
    handle: Option<JoinHandle<()>>,
}

#[derive(GlobalEvent)]
pub struct RendererProfileFrame {
    pub fps: ProfileFrame,
    pub view_tick: ProfileFrame,
    pub events: ProfileFrame,
    pub backend_tick: ProfileFrame,
    pub drawn_primitives: ProfileFrame,
    pub draw_calls: ProfileFrame,
}

trait RendererProfilerTrait {
    fn view_tick_start(&self) {}
    fn view_tick_end(&self) {}
    fn evens_start(&self) {}
    fn evens_end(&self) {}
    fn backend_tick_start(&self) {}
    fn backend_tick_end(&self) {}
    fn draw_result(&self, _execute_result: PassExecuteResult) {}
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
    evens: PeriodProfiler,
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

    fn evens_start(&self) {
        self.evens.start();
    }

    fn evens_end(&self) {
        self.evens.end();
    }

    fn backend_tick_start(&self) {
        self.backend_tick.start();
    }

    fn backend_tick_end(&self) {
        self.backend_tick.end();
    }

    fn draw_result(&self, execute_result: PassExecuteResult) {
        self.drawn_primitives
            .tick(execute_result.primitives() as u32);
        self.draw_calls.tick(execute_result.draw_calls() as u32);
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
                        events: self.evens.get_frame(),
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
            evens: PeriodProfiler::new(0.5),
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

impl<E: Copy> Drop for Renderer<E> {
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

impl<E> Renderer<E>
where
    E: 'static + Copy + Send + Sync + Sized,
{
    pub fn new<C>(
        view_config: ViewConfig,
        backend_config: RendererBackendConfig<C, E>,
        use_profiling: bool,
    ) -> Result<Self, RendererError>
    where
        C: ChainExecute<E> + Send + Sync + 'static,
    {
        if use_profiling {
            Self::new_inner(view_config, backend_config, RendererProfiler::new())
        } else {
            Self::new_inner(view_config, backend_config, DummyRendererProfiler {})
        }
    }

    fn new_inner<P, C>(
        view_config: ViewConfig,
        backend_config: RendererBackendConfig<C, E>,
        profiler: P,
    ) -> Result<Self, RendererError>
    where
        P: RendererProfilerTrait + Send + Sync + 'static,
        C: ChainExecute<E> + Send + Sync + 'static,
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
        let renderer_queue = Arc::new(ArrayQueue::<RenderPassEvent<E>>::new(
            RENDERER_QUEUE_CAPACITY,
        ));
        let renderer_queue_clone = renderer_queue.clone();
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
                    renderer_queue_clone,
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
            renderer_queue,
            profile_frames,
            handle: Some(handle),
        })
    }

    fn renderer<P, C>(
        view_config: ViewConfig,
        backend_config: RendererBackendConfig<C, E>,
        inputs_sender: Arc<ArrayQueue<InputEvent>>,
        renderer_queue: Arc<ArrayQueue<RenderPassEvent<E>>>,
        mut renderables_buffer: Output<Vec<Renderable>>,
        profiler: Arc<P>,
        stop_signal: Arc<AtomicBool>,
    ) -> Result<(), RendererError>
    where
        P: RendererProfilerTrait + Send + Sync + 'static,
        E: 'static + Copy,
        C: ChainExecute<E> + Send + Sync + 'static,
    {
        let mut view =
            View::open(view_config, inputs_sender).map_err(RendererError::ViewCreateError)?;

        let mut backend = RendererBackend::new(backend_config, view.get_handle())
            .map_err(RendererError::BackendCreateError)?;

        info!("Renderer thread started");
        let mut result = Ok(());
        while !stop_signal.load(Ordering::SeqCst) {
            // Process View. Usually this will produce input events
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

            // Process render pass events from the ECS
            profiler.evens_start();
            while let Some(event) = renderer_queue.pop() {
                // Dispatch the event to the backend
                if let Err(e) = backend.dispatch_event(&event) {
                    warn!("Failed to dispatch render pass event: {:?}", e);
                    result = Err(RendererError::BackendTickError(e));
                    break;
                }
            }
            profiler.evens_end();

            // Render the frame
            profiler.backend_tick_start();
            let renderables = renderables_buffer.read();
            match backend.render(renderables.as_slice()) {
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

    /// After attaching the renderer to the ECS, it will automatically collect the renderables
    /// and send them to the renderer thread (see `renderable` mod for more details).
    ///
    /// When any input event is received, it will be sent to the ECS as `InputEvent` events.
    /// It will capture all user's render pass events as `RenderPassEvent<E>` events and
    /// send them to the renderer thread for processing.
    /// Also, if you enabled profiling, it will send profiling data as `RendererProfileFrame`
    /// events to the ECS every second.
    /// Additionally, if the Window or Renderer is closed/failed the event loop will be stopped
    /// by sending a `StopEventLoop` event to the ECS.
    ///
    /// This function moves the renderer into the ECS world.
    pub fn attach_to_ecs(self, world: &mut World) {
        #[derive(Component)]
        struct Boxed {
            raw: NonNull<()>,
        }
        impl Boxed {
            fn new<E>(renderer: Renderer<E>) -> Self
            where
                E: 'static + Copy + Send + Sync + Sized,
            {
                let raw =
                    unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(renderer)) as *mut ()) };
                Boxed { raw }
            }

            fn deref<E>(&self) -> &Renderer<E>
            where
                E: 'static + Copy + Send + Sync + Sized,
            {
                // SAFETY: We are guaranteed that the raw pointer is valid
                // and points to a Renderer<E> because we created it from a Box<Renderer<E>>.
                unsafe { &*(self.raw.as_ptr() as *const Renderer<E>) }
            }

            fn deref_mut<E>(&mut self) -> &mut Renderer<E>
            where
                E: 'static + Copy + Send + Sync + Sized,
            {
                // SAFETY: We are guaranteed that the raw pointer is valid
                // and points to a Renderer<E> because we created it from a Box<Renderer<E>>.
                unsafe { &mut *(self.raw.as_ptr() as *mut Renderer<E>) }
            }
        }

        impl Drop for Boxed {
            fn drop(&mut self) {
                info!("Dropping renderer box");

                // SAFETY: We are guaranteed that the raw pointer is valid
                // and points to a Renderer<E> because we created it from a Box<Renderer<E>>.
                unsafe { Box::from_raw(self.raw.as_ptr() as *mut Renderer<()>) };
            }
        }

        // Setup the renderer player entity in the ECS
        let renderer_entity = world.spawn();
        world.insert(renderer_entity, Boxed::new(self));

        // If renderer loop is closed or stopped,
        // we need to stop the event loop
        fn view_closed_handler<E>(
            _: Receiver<Tick>,
            renderer: Single<&Boxed>,
            mut sender: Sender<StopEventLoop>,
        ) where
            E: 'static + Copy + Send + Sync + Sized,
        {
            // Check if the view was closed, if so, send a global event to stop the event loop
            let renderer = renderer.deref::<E>();
            if renderer.stop_signal.load(Ordering::Relaxed) {
                info!("View closed, stopping the event loop");
                sender.send(StopEventLoop);
            }
        }

        // Check if there's any profile frame to process.
        // If so, push them to the ECS
        fn profile_handler<E>(
            _: Receiver<Tick>,
            renderer: Single<&Boxed>,
            mut sender: Sender<RendererProfileFrame>,
        ) where
            E: 'static + Copy + Send + Sync + Sized,
        {
            let renderer = renderer.deref::<E>();
            while let Some(frame) = renderer.profile_frames.pop() {
                sender.send(frame);
            }
        }

        // Check if there's any input event to process.
        // If so, push them to the ECS
        fn inputs_handler<E>(
            _: Receiver<Tick>,
            renderer: Single<&Boxed>,
            mut sender: Sender<InputEvent>,
        ) where
            E: 'static + Copy + Send + Sync + Sized,
        {
            let renderer = renderer.deref::<E>();
            while let Some(input) = renderer.inputs_queue.pop() {
                sender.send(input);
            }
        }

        // Transfer render pass events from the ECS to the renderer thread
        fn render_pass_event_handler<E>(rpe: Receiver<RenderPassEvent<E>>, renderer: Single<&Boxed>)
        where
            E: 'static + Copy + Send + Sync + Sized,
        {
            let renderer = renderer.deref::<E>();
            let _ = renderer.renderer_queue.push(rpe.event.clone());
        }

        #[derive(Query)]
        struct Query<'a> {
            mesh: &'a RenderableMesh,
            position: Option<&'a Position>,
            rotation: Option<&'a Rotation>,
            scale: Option<&'a Scale>,
            material: Option<&'a Material>,
        }

        // Collect renderables from the ECS and send them to the renderer thread
        // This function will be called every tick to collect the renderables
        // and send them to the renderer thread.
        fn collect_renderables<E>(
            _: Receiver<Tick>,
            mut renderer: Single<&mut Boxed>,
            fetcher: Fetcher<Query>,
        ) where
            E: 'static + Copy + Send + Sync + Sized,
        {
            // TODO: Do not allocate a new vector every time, instead use a static one!
            let renderer = renderer.deref_mut::<E>();
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
            renderer.renderables_buffer_input.write(renderables);
        }

        world.add_handler(profile_handler::<E>);
        world.add_handler(inputs_handler::<E>);
        world.add_handler(view_closed_handler::<E>);
        world.add_handler(collect_renderables::<E>);
        world.add_handler(render_pass_event_handler::<E>);
    }
}
