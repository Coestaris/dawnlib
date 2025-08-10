use crate::passes::events::PassEventTarget;
use crate::passes::result::PassExecuteResult;
use crate::renderable::Renderable;

pub mod chain;
pub mod events;
pub mod pipeline;
pub mod result;

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

pub(crate) struct PassExecuteContext<'a> {
    pub(crate) renderables: &'a [Renderable],
}

impl<'a> PassExecuteContext<'a> {
    pub fn new(renderables: &'a [Renderable]) -> Self {
        PassExecuteContext { renderables }
    }

    /// Executes the render pass on using the current context.
    pub fn execute<E, P>(&self, pass: &mut P) -> PassExecuteResult
    where
        E: Copy + 'static,
        P: RenderPass<E>,
    {
        // TODO: Profiling?
        pass.begin();
        for renderable in self.renderables {
            // TODO: Iterate over meshes if needed
            pass.on_renderable(renderable);
            pass.on_mesh(renderable.mesh_id);
        }
        pass.end();

        PassExecuteResult::new(2, self.renderables.len())
    }
}
