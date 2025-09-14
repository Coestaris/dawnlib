use crate::passes::events::{PassEventTarget, PassEventTrait};
use crate::passes::result::RenderResult;
use crate::renderable::Renderable;
use crate::renderer::backend::RendererBackend;
use crate::renderer::DataStreamFrame;
use web_time::Duration;

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

pub struct ChainExecuteCtx<'a, E: PassEventTrait> {
    // The renderables to be processed by the render pass.
    pub(crate) frame: &'a DataStreamFrame,
    // Amount of time consumed by render pass in the chain.
    pub(crate) durations: [Duration; MAX_RENDER_PASSES],
    // The renderer backend context
    pub(crate) backend: &'a mut RendererBackend<E>,
}

impl<'a, E: PassEventTrait> ChainExecuteCtx<'a, E> {
    pub fn new(frame: &'a DataStreamFrame, backend: &'a mut RendererBackend<E>) -> Self {
        ChainExecuteCtx {
            frame,
            durations: [Duration::ZERO; MAX_RENDER_PASSES],
            backend,
        }
    }

    /// Executes the render pass on using the current context.
    pub fn execute<P>(&mut self, idx: usize, pass: &mut P) -> RenderResult
    where
        E: PassEventTrait,
        P: RenderPass<E>,
    {
        let start = web_time::Instant::now();

        let mut result = RenderResult::default();
        result += pass.begin(self.backend, self.frame);
        for renderable in self.frame.renderables.iter() {
            result += pass.on_renderable(self.backend, renderable);
        }
        result += pass.end(self.backend);

        let elapsed = start.elapsed();
        self.durations[idx] = elapsed;

        result
    }
}
