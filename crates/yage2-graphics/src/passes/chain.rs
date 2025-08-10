use crate::passes::events::PassEventTarget;
use crate::passes::result::PassExecuteResult;
use crate::passes::{PassExecuteContext, RenderPass};
use std::marker::PhantomData;

// Compile-time Heterogeneous List (HList) for Render Passes
// Lisp is Love. Lisp is Life.
pub struct ChainNil<E>
where
    E: Copy + 'static,
{
    _marker: PhantomData<E>,
}

impl<E> ChainNil<E>
where
    E: Copy + 'static,
{
    pub fn new() -> Self {
        ChainNil {
            _marker: Default::default(),
        }
    }
}

pub struct ChainCons<E, H, T>
where
    E: Copy + 'static,
{
    _marker: PhantomData<E>,
    head: Box<H>,
    tail: T,
}

impl<E, H, T> ChainCons<E, H, T>
where
    E: Copy + 'static,
{
    pub fn new(head: H, tail: T) -> Self {
        ChainCons {
            _marker: Default::default(),
            head: Box::new(head),
            tail,
        }
    }
}

pub trait ChainExecute<E> {
    fn execute(&mut self, ctx: &PassExecuteContext) -> PassExecuteResult;
    fn get_targets(&self) -> Vec<PassEventTarget<E>>;
}

// Nil is the dead-end. Doing nothing.
impl<E> ChainExecute<E> for ChainNil<E>
where
    E: Copy + 'static,
{
    #[inline(always)]
    fn execute(&mut self, _: &PassExecuteContext) -> PassExecuteResult {
        // No operation, as this is the end of the chain.
        PassExecuteResult::default()
    }
    #[inline(always)]
    fn get_targets(&self) -> Vec<PassEventTarget<E>> {
        // No targets, as this is the end of the chain.
        vec![]
    }
}

// Cons is the recursive case.
// It runs the head pass and then recurses on the tail.
impl<E, H, T> ChainExecute<E> for ChainCons<E, H, T>
where
    E: Copy + 'static,
    H: RenderPass<E> + Send + Sync + 'static,
    T: ChainExecute<E>,
{
    #[inline(always)]
    fn execute(&mut self, ctx: &PassExecuteContext) -> PassExecuteResult {
        // Execute the head pass.
        // This will handle all required operations for the head pass
        let mut result = ctx.execute::<E, H>(&mut self.head);

        // Continue with the next pass in the chain.
        // Accumulate the results from the tail.
        result += self.tail.execute(ctx);
        result
    }

    #[inline(always)]
    fn get_targets(&self) -> Vec<PassEventTarget<E>> {
        // Collect targets from the head and tail passes.
        let mut targets = self.head.get_target();
        targets.extend(self.tail.get_targets());
        targets
    }
}

/// Contracts a heterogeneous list of render passes.
/// This macro allows you to create a chain of render passes
/// using a simple syntax.
#[macro_export]
macro_rules! construct_chain {
    () => { ChainNil::new() };
    ($head:expr $(, $tail:expr)* $(,)?) => { ChainCons::new($head, construct_chain!($($tail),*)) };
}
