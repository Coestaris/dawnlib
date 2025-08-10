use crate::renderable::Renderable;

pub trait RenderPass {
    fn name(&self) -> &str;
    #[inline(always)]
    fn begin(&mut self) {}

    #[inline(always)]
    fn end(&mut self) {}

    #[inline(always)]
    fn on_renderable(&mut self, node: &Renderable) {}

    #[inline(always)]
    fn on_mesh(&mut self, mesh: u32) {}
}

// Compile-time Heterogeneous List (HList) for Render Passes
// Lisp is the best.
pub struct ChainNil;
pub struct ChainCons<H, T> {
    head: Box<H>,
    tail: T,
}

impl<H, T> ChainCons<H, T> {
    pub fn new(head: H, tail: T) -> Self {
        ChainCons {
            head: Box::new(head),
            tail,
        }
    }
}

pub(crate) struct ChainExecuteContext<'a> {
    pub(crate) renderables: &'a [Renderable],
}

pub(crate) trait ChainExecute {
    fn execute(&mut self, ctx: &ChainExecuteContext);
}

// Nil is the dead-end. Doing nothing.
impl ChainExecute for ChainNil {
    #[inline(always)]
    fn execute(&mut self, _: &ChainExecuteContext) {}
}

// Cons is the recursive case.
// It runs the head pass and then recurses on the tail.
impl<H, T> ChainExecute for ChainCons<H, T>
where
    H: RenderPass,
    T: ChainExecute,
{
    #[inline(always)]
    fn execute(&mut self, ctx: &ChainExecuteContext) {
        // Process the render pass
        self.head.begin();
        for renderable in ctx.renderables {
            self.head.on_renderable(renderable);
            // for mesh in &node.meshes {
            self.head.on_mesh(renderable.mesh_id);
            // }
        }
        self.head.end();

        // Continue with the next pass in the chain.
        self.tail.execute(ctx);
    }
}

/// Contracts a heterogeneous list of render passes.
/// This macro allows you to create a chain of render passes
/// using a simple syntax.
#[macro_export]
macro_rules! construct_chain {
    () => { ChainNil };
    ($head:expr $(, $tail:expr)* $(,)?) => { ChainCons::new($head, construct_chain!($($tail),*)) };
}

