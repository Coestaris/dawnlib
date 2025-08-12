use crate::passes::events::{PassEventTarget, PassEventTrait};
use crate::passes::result::PassExecuteResult;
use crate::renderable::Renderable;
use std::time::Duration;

pub mod chain;
pub mod events;
pub mod pipeline;
pub mod result;

pub(crate) const MAX_RENDER_PASSES: usize = 32;

pub trait RenderPass<E: PassEventTrait>: Send + Sync + 'static {
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
    fn dispatch(&mut self, _event: &E) {
        // The default implementation does nothing.
    }

    /// Get the name of the render pass.
    fn name(&self) -> &str;

    /// Begin the render pass execution.
    /// This method is called before processing any renderables or meshes.
    #[inline(always)]
    fn begin(&mut self) -> PassExecuteResult {
        PassExecuteResult::default()
    }

    /// End the render pass execution.
    /// This method is called after processing all renderables and meshes.
    #[inline(always)]
    fn end(&mut self) -> PassExecuteResult {
        PassExecuteResult::default()
    }

    /// Process a renderable object.
    #[inline(always)]
    fn on_renderable(&mut self, _renderable: &Renderable) -> PassExecuteResult {
        PassExecuteResult::default()
    }

    /// This method is called for each mesh in the renderable.
    #[inline(always)]
    fn on_mesh(&mut self, _mesh: u32) -> PassExecuteResult {
        PassExecuteResult::default()
    }
}

pub(crate) struct ChainExecuteCtx<'a> {
    // The renderables to be processed by the render pass.
    pub(crate) renderables: &'a [Renderable],
    // Amount of time consumed by render pass in the chain.
    pub(crate) durations: [Duration; MAX_RENDER_PASSES],
}

impl<'a> ChainExecuteCtx<'a> {
    pub fn new(renderables: &'a [Renderable]) -> Self {
        ChainExecuteCtx {
            renderables,
            durations: [Duration::ZERO; MAX_RENDER_PASSES],
        }
    }

    /// Executes the render pass on using the current context.
    pub fn execute<E, P>(&mut self, idx: usize, pass: &mut P) -> PassExecuteResult
    where
        E: PassEventTrait,
        P: RenderPass<E>,
    {
        let start = std::time::Instant::now();

        let mut result = PassExecuteResult::default();
        result += pass.begin();
        for renderable in self.renderables {
            result += pass.on_renderable(renderable);

            // TODO: Iterate over meshes when renderable has multiple meshes.
            //       For now, we assume each renderable has only one mesh.
            result += pass.on_mesh(renderable.mesh_id);
        }
        result += pass.end();

        let elapsed = start.elapsed();
        self.durations[idx] = elapsed;

        result
    }
}
