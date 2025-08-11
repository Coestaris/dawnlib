use crate::passes::events::PassEventTarget;
use crate::passes::result::PassExecuteResult;
use crate::renderable::Renderable;
use std::time::Duration;

pub mod chain;
pub mod events;
pub mod pipeline;
pub mod result;

pub(crate) const MAX_RENDER_PASSES: usize = 32;

pub trait RenderPass<E>
where
    E: Copy + 'static,
{
    #[inline(always)]
    fn get_target(&self) -> Vec<PassEventTarget<E>> {
        // Default implementation returns an empty vector.
        vec![]
    }

    fn dispatch(&mut self, _event: &E) {
        // Default implementation does nothing.
    }

    fn name(&self) -> &str;

    #[inline(always)]
    fn begin(&mut self) {}

    #[inline(always)]
    fn end(&mut self) {}

    #[inline(always)]
    fn on_renderable(&mut self, _renderable: &Renderable) -> PassExecuteResult {
        // Default implementation does nothing and returns a default result.
        PassExecuteResult::default()
    }

    #[inline(always)]
    fn on_mesh(&mut self, _mesh: u32) -> PassExecuteResult {
        // Default implementation does nothing and returns a default result.
        PassExecuteResult::default()
    }
}

pub(crate) struct ChainExecuteCtx<'a> {
    pub(crate) renderables: &'a [Renderable],

    // Amount of time consumed by each renderable in the pass.
    // TODO: What if more than passes are executed?
    pub(crate) profile: [Duration; MAX_RENDER_PASSES],
}

impl<'a> ChainExecuteCtx<'a> {
    pub fn new(renderables: &'a [Renderable]) -> Self {
        ChainExecuteCtx {
            renderables,
            profile: [Duration::ZERO; MAX_RENDER_PASSES],
        }
    }

    /// Executes the render pass on using the current context.
    pub fn execute<E, P>(&mut self, idx: usize, pass: &mut P) -> PassExecuteResult
    where
        E: Copy + 'static,
        P: RenderPass<E>,
    {
        let start = std::time::Instant::now();
        pass.begin();
        let mut result = PassExecuteResult::default();
        for renderable in self.renderables {
            // TODO: Iterate over meshes if needed
            result += pass.on_renderable(renderable);
            result += pass.on_mesh(renderable.mesh_id);
        }
        pass.end();
        let elapsed = start.elapsed();
        self.profile[idx] = elapsed;
        result
    }
}
