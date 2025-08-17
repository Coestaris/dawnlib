use crate::passes::events::{PassEventTarget, PassEventTrait};
use crate::passes::result::PassExecuteResult;
use crate::passes::{ChainExecuteCtx, RenderPass};
use std::marker::PhantomData;

// Compile-time Heterogeneous List (HList) for Render Passes
// Lisp is Love. Lisp is Life.
pub struct ChainNil<E: PassEventTrait> {
    _marker: PhantomData<E>,
}

impl<E: PassEventTrait> ChainNil<E> {
    pub fn new() -> Self {
        ChainNil {
            _marker: Default::default(),
        }
    }
}

pub struct ChainCons<E: PassEventTrait, H, T> {
    _marker: PhantomData<E>,
    head: Box<H>,
    tail: T,
}

impl<E: PassEventTrait, H, T> ChainCons<E, H, T> {
    pub fn new(head: H, tail: T) -> Self {
        ChainCons {
            _marker: Default::default(),
            head: Box::new(head),
            tail,
        }
    }
}

/// A trait for optimization-friendly processing a chain (HList) of render passes.
pub trait RenderChain<E: PassEventTrait> {
    /// Sequentially execute the chain of render passes.
    #[inline(always)]
    fn execute(&mut self, _: usize, _: &mut ChainExecuteCtx<E>) -> PassExecuteResult {
        PassExecuteResult::default()
    }

    /// Get the length of the chain.
    #[inline(always)]
    fn length(&self) -> usize {
        0
    }

    /// Collect all targets from the chain.
    #[inline(always)]
    fn get_targets(&self) -> Vec<PassEventTarget<E>> {
        vec![]
    }

    /// Collect all names from the chain.
    #[inline(always)]
    fn get_names(&self) -> Vec<&str> {
        vec![]
    }
}

// Nil is the dead-end. Doing nothing.
impl<E: PassEventTrait> RenderChain<E> for ChainNil<E> {}

// Cons is the recursive case.
// It runs the head pass and then recurses on the tail.
impl<E: PassEventTrait, H, T> RenderChain<E> for ChainCons<E, H, T>
where
    H: RenderPass<E>,
    T: RenderChain<E>,
{
    #[inline(always)]
    fn execute(&mut self, idx: usize, ctx: &mut ChainExecuteCtx<E>) -> PassExecuteResult {
        // Execute the head pass.
        // This will handle all required operations for the head pass
        let mut result = ctx.execute::<H>(idx, &mut self.head);

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
///
/// # Example:
/// ```
/// use dawn_graphics::passes::RenderPass;
///
/// struct PassA;
/// struct PassB;
/// struct PassC;
///
/// struct Event;
///
/// impl RenderPass<Event> for PassA {
///    fn name(&self) -> &str { "PassA" }
/// }
/// impl RenderPass<Event> for PassB {
///   fn name(&self) -> &str { "PassB" }
/// }
/// impl RenderPass<Event> for PassC {
///   fn name(&self) -> &str { "PassC" }
/// }
///
/// use dawn_graphics::construct_chain;
/// let chain = construct_chain!(PassA, PassB, PassC);
/// ```
#[macro_export]
macro_rules! construct_chain {
    () => { ChainNil::new() };
    ($head:expr $(, $tail:expr)* $(,)?) => { ChainCons::new($head, construct_chain!($($tail),*)) };
}
