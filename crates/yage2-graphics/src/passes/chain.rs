use crate::passes::events::PassEventTarget;
use crate::passes::result::PassExecuteResult;
use crate::passes::{ChainExecuteCtx, RenderPass};
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

pub trait RenderChain<E> {
    #[inline(always)]
    fn execute(&mut self, _: usize, _: &mut ChainExecuteCtx) -> PassExecuteResult {
        PassExecuteResult::default()
    }

    #[inline(always)]
    fn length(&self) -> usize {
        0
    }

    #[inline(always)]
    fn get_targets(&self) -> Vec<PassEventTarget<E>> {
        vec![]
    }

    #[inline(always)]
    fn get_names(&self) -> Vec<&str> {
        vec![]
    }
}

// Nil is the dead-end. Doing nothing.
impl<E> RenderChain<E> for ChainNil<E> where E: Copy + 'static {}

// Cons is the recursive case.
// It runs the head pass and then recurses on the tail.
impl<E, H, T> RenderChain<E> for ChainCons<E, H, T>
where
    E: Copy + 'static,
    H: RenderPass<E> + Send + Sync + 'static,
    T: RenderChain<E>,
{
    #[inline(always)]
    fn execute(&mut self, idx: usize, ctx: &mut ChainExecuteCtx) -> PassExecuteResult {
        // Execute the head pass.
        // This will handle all required operations for the head pass
        let mut result = ctx.execute::<E, H>(idx, &mut self.head);

        // Continue with the next pass in the chain.
        // Accumulate the results from the tail.
        result += self.tail.execute(idx + 1, ctx);
        result
    }

    #[inline(always)]
    fn length(&self) -> usize {
        // Count the head pass and add the count of the tail.
        1 + self.tail.length()
    }

    #[inline(always)]
    fn get_targets(&self) -> Vec<PassEventTarget<E>> {
        // Collect targets from the head and tail passes.
        let mut targets = self.head.get_target();
        targets.extend(self.tail.get_targets());
        targets
    }

    #[inline(always)]
    fn get_names(&self) -> Vec<&str> {
        // Collect names from the head and tail passes.
        let mut names = vec![self.head.name()];
        names.extend(self.tail.get_names());
        names
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
