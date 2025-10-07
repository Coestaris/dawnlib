use crate::gl::timer::GPUTimer;
use crate::passes::events::{PassEventTarget, PassEventTrait};
use crate::passes::result::RenderResult;
use crate::renderable::Renderable;
use crate::renderer::backend::RendererBackend;
use crate::renderer::DataStreamFrame;
use dawn_util::profile::Stopwatch;
use std::sync::Arc;

pub mod chain;
pub mod events;
pub mod pipeline;
pub mod result;

pub(crate) const MAX_RENDER_PASSES: usize = 32;

pub trait RenderPass<E: PassEventTrait>: 'static {
    /// Declare the targets for this render pass.
    /// This is used to address events that are relevant to this pass.
    #[inline(always)]
    fn get_target(&self) -> Vec<PassEventTarget<E>> {
        // Default implementation returns an empty vector.
        vec![]
    }

    /// Dispatch an asynchronous event to the render pass.
    /// This method is called when an event is received that is relevant
    /// to declared targets.
    #[inline(always)]
    fn dispatch(&mut self, _event: E) {
        // The default implementation does nothing.
    }

    /// Get the name of the render pass.
    fn name(&self) -> &str;

    /// Begin the render pass execution.
    /// This method is called before processing any renderables or meshes.
    #[inline(always)]
    fn begin(&mut self, _backend: &RendererBackend<E>, _frame: &DataStreamFrame) -> RenderResult {
        RenderResult::default()
    }

    /// Process a renderable object.
    #[inline(always)]
    fn on_renderable(
        &mut self,
        _backend: &mut RendererBackend<E>,
        _renderable: &Renderable,
    ) -> RenderResult {
        RenderResult::default()
    }

    /// End the render pass execution.
    /// This method is called after processing all renderables and meshes.
    #[inline(always)]
    fn end(&mut self, _backend: &mut RendererBackend<E>) -> RenderResult {
        RenderResult::default()
    }
}

pub struct ChainTimers {
    // The CPU timers for each render pass in the chain.
    pub cpu: [Stopwatch; MAX_RENDER_PASSES],
    // The GPU timers for each render pass in the chain.
    pub gpu: [GPUTimer; MAX_RENDER_PASSES],
}

impl ChainTimers {
    pub fn new(cpu_wma: f32, gl: Arc<glow::Context>) -> Self {
        let cpu = array_init::array_init(|_| Stopwatch::new(cpu_wma));
        let gpu = array_init::array_init(|_| GPUTimer::new(gl.clone()).unwrap());
        ChainTimers { cpu, gpu }
    }
}

pub struct ChainExecuteCtx<'a, E: PassEventTrait> {
    // The renderables to be processed by the render pass.
    pub(crate) frame: &'a DataStreamFrame,
    // The timers for each render pass in the chain.
    pub(crate) timers: &'a mut ChainTimers,
    // The renderer backend context
    pub(crate) backend: &'a mut RendererBackend<E>,
}

impl<'a, E: PassEventTrait> ChainExecuteCtx<'a, E> {
    pub fn new(
        frame: &'a DataStreamFrame,
        backend: &'a mut RendererBackend<E>,
        timers: &'a mut ChainTimers,
    ) -> Self {
        ChainExecuteCtx {
            frame,
            timers,
            backend,
        }
    }

    /// Executes the render pass on using the current context.
    pub fn execute<P>(&mut self, idx: usize, pass: &mut P) -> RenderResult
    where
        E: PassEventTrait,
        P: RenderPass<E>,
    {
        self.timers.cpu[idx].start();
        self.timers.gpu[idx].start();

        let mut result = RenderResult::default();
        result += pass.begin(self.backend, self.frame);
        for renderable in self.frame.renderables.iter() {
            result += pass.on_renderable(self.backend, renderable);
        }
        result += pass.end(self.backend);

        self.timers.gpu[idx].stop();
        self.timers.cpu[idx].stop();

        result
    }
}
