use crate::input::InputEvent;
use crate::passes::chain::RenderChain;
use crate::passes::events::RenderPassEvent;
use crate::passes::pipeline::RenderPipeline;
use crate::passes::result::PassExecuteResult;
use crate::passes::{ChainExecuteCtx, MAX_RENDER_PASSES};
use crate::renderable::{Material, Position, Renderable, RenderableMesh, Rotation, Scale};
use crate::view::{TickResult, View, ViewConfig, ViewError, ViewHandle, ViewTrait};
use crossbeam_queue::ArrayQueue;
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver, Sender};
use evenio::fetch::{Fetcher, Single};
use evenio::query::Query;
use evenio::world::World;
use glam::Vec3;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use std::time::Duration;
use triple_buffer::{triple_buffer, Input, Output};
use yage2_core::ecs::{StopEventLoop, Tick};
use yage2_core::profile::{PeriodProfiler, ProfileFrame, TickProfiler};

pub(crate) trait RendererBackendTrait<E>
where
    E: Copy + 'static,
    Self: Sized,
{
    fn new(
        config: RendererBackendConfig,
        view_handle: ViewHandle,
    ) -> Result<Self, RendererBackendError>;

    fn before_frame(&mut self) -> Result<(), RendererBackendError>;
    fn after_frame(&mut self) -> Result<(), RendererBackendError>;
}

#[cfg(feature = "gl")]
mod backend_impl {
    pub type RendererBackend<E> = crate::gl::GLRenderer<E>;
    pub type RendererBackendConfig = crate::gl::GLRendererConfig;
    pub type RendererBackendError = crate::gl::GLRendererError;
}

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
    /// Actual number of frames drawn per second.
    pub fps: ProfileFrame,

    /// The time spend on the processing of the view tick
    /// (OS-dependent Window handling).
    pub view_tick: ProfileFrame,

    /// The time spend on processing the events from the ECS
    /// by the renderer pipeline.
    pub events: ProfileFrame,

    /// The time spend no the actual rendering of the frame
    /// (including the time spend on the backend).
    pub render: ProfileFrame,

    /// The approximate number of primitives drawn
    /// (triangles, lines, points, etc.) in the frame.
    pub drawn_primitives: ProfileFrame,

    /// The number of draw calls made in the frame.
    /// This is the number of times the GPU was instructed
    pub draw_calls: ProfileFrame,

    /// The amount of time spend on each render pass.
    /// The key is the name of the pass, and the value is the time spent on it.
    /// This is used for profiling the render passes.
    pub pass_profile: HashMap<String, ProfileFrame>,
}

trait RendererProfilerTrait {
    fn set_queue(&mut self, queue: Arc<ArrayQueue<RendererProfileFrame>>) {}
    fn set_pass_names(&mut self, names: &[&str]) {}
    fn view_tick_start(&mut self) {}
    fn view_tick_end(&mut self) {}
    fn evens_start(&mut self) {}
    fn evens_end(&mut self) {}
    fn render_start(&mut self) {}
    fn render_end(
        &mut self,
        execute_result: PassExecuteResult,
        prof: &[Duration; MAX_RENDER_PASSES],
    ) {
    }
}

struct RendererProfiler {
    fps: TickProfiler,
    view_tick: PeriodProfiler,
    events: PeriodProfiler,
    render: PeriodProfiler,
    draw_calls: TickProfiler,
    drawn_primitives: TickProfiler,
    queue: Option<Arc<ArrayQueue<RendererProfileFrame>>>,
    pass_names: Vec<String>,
    last_send: std::time::Instant,
}

impl RendererProfilerTrait for RendererProfiler {
    fn set_queue(&mut self, queue: Arc<ArrayQueue<RendererProfileFrame>>) {
        // TODO: Implement thread-unsafe profiling
        self.queue = Some(queue);
    }

    fn set_pass_names(&mut self, names: &[&str]) {
        self.pass_names.clear();
        for name in names {
            self.pass_names.push(name.to_string());
        }
        debug!("Renderer profiler pass names set: {:?}", self.pass_names);
    }

    fn view_tick_start(&mut self) {
        self.fps.tick(1);
        self.view_tick.start();
    }

    fn view_tick_end(&mut self) {
        self.view_tick.end();
    }

    fn evens_start(&mut self) {
        self.events.start();
    }

    fn evens_end(&mut self) {
        self.events.end();
    }

    fn render_start(&mut self) {
        self.render.start();
    }

    fn render_end(
        &mut self,
        execute_result: PassExecuteResult,
        prof: &[Duration; MAX_RENDER_PASSES],
    ) {
        self.render.end();

        self.drawn_primitives
            .tick(execute_result.primitives().unwrap() as u32);
        self.draw_calls
            .tick(execute_result.draw_calls().unwrap() as u32);

        // Call these each second
        let now = std::time::Instant::now();
        if now.duration_since(self.last_send) >= Duration::from_secs(1) {
            self.last_send = now;

            self.fps.update();
            self.drawn_primitives.update();

            let mut pass_profile = HashMap::with_capacity(self.pass_names.len());
            for (i, name) in self.pass_names.iter().enumerate() {
                if i < prof.len() {
                    let ms = prof[i].as_millis() as f32;
                    // TODO: Implement some kind of smoothing
                    pass_profile.insert(name.clone(), ProfileFrame::new(ms, ms, ms));
                }
            }
            let frame = RendererProfileFrame {
                fps: self.fps.get_frame(),
                view_tick: self.view_tick.get_frame(),
                events: self.events.get_frame(),
                render: self.render.get_frame(),
                drawn_primitives: self.drawn_primitives.get_frame(),
                draw_calls: self.draw_calls.get_frame(),
                pass_profile,
            };

            if let Some(queue) = &self.queue {
                if queue.push(frame).is_err() {
                    warn!("Renderer profiler queue is full, dropping frame");
                }
            } else {
                warn!("Renderer profiler queue is not set, dropping frame");
            }
        }
    }
}
impl RendererProfiler {
    pub fn new() -> Self {
        RendererProfiler {
            fps: TickProfiler::new(0.5),
            view_tick: PeriodProfiler::new(0.5),
            events: PeriodProfiler::new(0.5),
            render: PeriodProfiler::new(0.5),
            drawn_primitives: TickProfiler::new(1.0),
            queue: None,
            draw_calls: TickProfiler::new(1.0),
            last_send: std::time::Instant::now(),
            pass_names: vec![],
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
    BackendRenderError(RendererBackendError),
    PipelineExecuteError(),
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
            RendererError::BackendRenderError(e) => write!(f, "Backend tick error: {}", e),
            RendererError::ProfilerSetupFailed => write!(f, "Failed to setup profiler"),
            RendererError::PipelineExecuteError() => write!(f, "Failed to execute render pipeline"),
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
        backend_config: RendererBackendConfig,
        constructor: impl FnOnce() -> RenderPipeline<C, E> + Send + Sync + 'static,
        use_profiling: bool,
    ) -> Result<Self, RendererError>
    where
        C: RenderChain<E> + Send + Sync + 'static,
    {
        if use_profiling {
            Self::new_inner(
                view_config,
                backend_config,
                constructor,
                RendererProfiler::new(),
            )
        } else {
            Self::new_inner(
                view_config,
                backend_config,
                constructor,
                DummyRendererProfiler {},
            )
        }
    }

    fn new_inner<P, C>(
        view_config: ViewConfig,
        backend_config: RendererBackendConfig,
        constructor: impl FnOnce() -> RenderPipeline<C, E> + Send + Sync + 'static,
        mut profiler: P,
    ) -> Result<Self, RendererError>
    where
        P: RendererProfilerTrait + Send + Sync + 'static,
        C: RenderChain<E> + Send + Sync + 'static,
    {
        // Setup profiler
        let stop_signal = Arc::new(AtomicBool::new(false));
        let profile_frames = Arc::new(ArrayQueue::<RendererProfileFrame>::new(
            PROFILE_QUEUE_CAPACITY,
        ));
        profiler.set_queue(profile_frames.clone());

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
                    constructor,
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
        backend_config: RendererBackendConfig,
        constructor: impl FnOnce() -> RenderPipeline<C, E> + Send + Sync + 'static,
        inputs_sender: Arc<ArrayQueue<InputEvent>>,
        renderer_queue: Arc<ArrayQueue<RenderPassEvent<E>>>,
        mut renderables_buffer: Output<Vec<Renderable>>,
        mut profiler: P,
        stop_signal: Arc<AtomicBool>,
    ) -> Result<(), RendererError>
    where
        P: RendererProfilerTrait + Send + Sync + 'static,
        E: 'static + Copy,
        C: RenderChain<E> + Send + Sync + 'static,
    {
        let mut view =
            View::open(view_config, inputs_sender).map_err(RendererError::ViewCreateError)?;
        let mut backend = RendererBackend::<E>::new(backend_config, view.get_handle())
            .map_err(RendererError::BackendCreateError)?;
        let mut pipeline = constructor();

        let pass_names = pipeline.get_names();
        profiler.set_pass_names(&pass_names);

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
                // Dispatch the event to the render pipeline
                pipeline.dispatch(&event);
            }
            profiler.evens_end();

            // Render the frame
            profiler.render_start();
            if let Err(e) = backend.before_frame() {
                error!("Failed to prepare backend for rendering: {:?}", e);
                result = Err(RendererError::BackendRenderError(e));
                break;
            }
            let renderables = renderables_buffer.read();
            let mut ctx = ChainExecuteCtx::new(renderables.as_slice());
            let pass_result = pipeline.execute(&mut ctx);
            if !pass_result.is_ok() {
                error!("Failed to execute render pipeline: {:?}", pass_result);
                result = Err(RendererError::PipelineExecuteError());
                break;
            }
            profiler.render_end(pass_result, &ctx.profile);

            // Do not include after frame in the profiler, because it usually synchronizes
            // the rendered frame with the OS by swapping buffer, that usually is synchronized
            // with the refresh rate of the display. So this will not be informative.
            if let Err(e) = backend.after_frame() {
                error!("Failed to finish backend rendering: {:?}", e);
                result = Err(RendererError::BackendRenderError(e));
                break;
            }
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
