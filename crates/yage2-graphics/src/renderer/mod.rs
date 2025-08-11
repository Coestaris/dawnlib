pub(crate) mod backend;
mod ecs;
mod profile;

use crate::input::InputEvent;
use crate::passes::chain::RenderChain;
use crate::passes::events::{PassEventTrait, RenderPassEvent};
use crate::passes::pipeline::RenderPipeline;
use crate::passes::result::PassExecuteResult;
use crate::passes::ChainExecuteCtx;
use crate::renderable::Renderable;
use crate::renderer::backend::{RendererBackend, RendererBackendError, RendererBackendTrait};
use crate::renderer::ecs::attach_to_ecs;
use crate::renderer::profile::{DummyRendererProfiler, RendererProfiler, RendererProfilerTrait};
use crate::view::{TickResult, View, ViewConfig, ViewError, ViewTrait};
use crossbeam_queue::ArrayQueue;
use evenio::component::Component;
use evenio::world::World;
use log::{info, warn};
use std::panic::UnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use triple_buffer::{triple_buffer, Input, Output};

// Re-export the necessary types for user
pub use backend::RendererBackendConfig;
pub use profile::RendererProfileFrame;

const INPUTS_QUEUE_CAPACITY: usize = 1024;
const RENDERER_QUEUE_CAPACITY: usize = 1024;
const PROFILE_QUEUE_CAPACITY: usize = 32;

#[derive(Component)]
pub struct Renderer<E: PassEventTrait> {
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

#[derive(Debug)]
pub enum RendererError {
    ViewCreateError(ViewError),
    RendererThreadSetupFailed,
    BackendCreateError(RendererBackendError),
    PipelineCreateError(String),
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

pub trait RenderChainConstructor<C, E> =
    FnOnce() -> Result<RenderPipeline<C, E>, String> + Send + Sync + 'static + UnwindSafe
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
    ///
    /// `use_profiling` is a flag that indicates whether to use profiling or not.
    /// It is only useful when connecting the renderer to the ECS.
    pub fn new<C>(
        view_config: ViewConfig,
        backend_config: RendererBackendConfig,
        constructor: impl RenderChainConstructor<C, E>,
        use_profiling: bool,
    ) -> Result<Self, RendererError>
    where
        C: RenderChain<E>,
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
        constructor: impl RenderChainConstructor<C, E>,
        mut profiler: P,
    ) -> Result<Self, RendererError>
    where
        P: RendererProfilerTrait,
        C: RenderChain<E>,
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
        let (renderables_buffer_input, mut renderables_buffer_output) =
            triple_buffer::<Vec<Renderable>>(&vec![]);
        let handle = Builder::new()
            .name("renderer".to_string())
            .spawn(move || {
                info!("Renderer thread started");

                let func = move || {
                    // Create the view, backend and the rendering pipeline
                    let mut view = View::open(view_config, inputs_queue_clone)
                        .map_err(RendererError::ViewCreateError)?;
                    let mut backend = RendererBackend::<E>::new(backend_config, view.get_handle())
                        .map_err(RendererError::BackendCreateError)?;
                    let mut pipeline = constructor().map_err(RendererError::PipelineCreateError)?;

                    // Notify the profiler about the pass names
                    let pass_names = pipeline.get_names();
                    profiler.set_pass_names(&pass_names);

                    info!("Starting renderer loop");
                    while !stop_signal_clone1.load(Ordering::SeqCst) {
                        match Self::handle_view(&mut view, &mut profiler) {
                            Ok(false) => {
                                return Ok(());
                            }
                            Err(e) => {
                                Err(e)?;
                            }
                            _ => {}
                        }
                        Self::handle_events(&mut profiler, &mut pipeline, &renderer_queue_clone)?;
                        Self::handle_render(
                            &mut profiler,
                            &mut backend,
                            &mut renderables_buffer_output,
                            &mut pipeline,
                        )?;
                    }

                    Ok(())
                };

                // TODO: Handle panics in the renderer thread
                let err: Result<(), RendererError> = func();

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

    #[inline(always)]
    fn handle_view(
        view: &mut View,
        profiler: &mut impl RendererProfilerTrait,
    ) -> Result<bool, RendererError> {
        // Process View. Usually this will produce input events
        profiler.view_tick_start();
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
        profiler.view_tick_end();
        Ok(true)
    }

    #[inline(always)]
    fn handle_events<C>(
        profiler: &mut impl RendererProfilerTrait,
        pipeline: &mut RenderPipeline<C, E>,
        renderer_queue: &ArrayQueue<RenderPassEvent<E>>,
    ) -> Result<(), RendererError>
    where
        C: RenderChain<E>,
    {
        profiler.evens_start();
        while let Some(event) = renderer_queue.pop() {
            pipeline.dispatch(&event);
        }
        profiler.evens_end();

        Ok(())
    }

    #[inline(always)]
    fn handle_render<C>(
        profiler: &mut impl RendererProfilerTrait,
        backend: &mut RendererBackend<E>,
        renderables_buffer: &mut Output<Vec<Renderable>>,
        pipeline: &mut RenderPipeline<C, E>,
    ) -> Result<(), RendererError>
    where
        C: RenderChain<E>,
    {
        profiler.render_start();
        if let Err(e) = backend.before_frame() {
            return Err(RendererError::BackendRenderError(e));
        }

        let renderables = renderables_buffer.read();
        let mut ctx = ChainExecuteCtx::new(renderables.as_slice());

        let pass_result = pipeline.execute(&mut ctx);
        if let PassExecuteResult::Failed = pass_result {
            return Err(RendererError::PipelineExecuteError());
        }

        // Do not include after frame in the profiler, because it usually synchronizes
        // the rendered frame with the OS by swapping buffer, that usually is synchronized
        // with the refresh rate of the display. So this will not be informative.
        profiler.render_end(pass_result, &ctx.profile);

        if let Err(e) = backend.after_frame() {
            return Err(RendererError::BackendRenderError(e))?;
        }

        Ok(())
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
        attach_to_ecs::<E>(self, world);
    }
}
