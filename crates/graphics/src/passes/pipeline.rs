use crate::passes::chain::RenderChain;
use crate::passes::events::{PassEventTarget, PassEventTrait, RenderPassEvent};
use crate::passes::result::RenderResult;
use crate::passes::{ChainExecuteCtx, MAX_RENDER_PASSES};
use std::mem::MaybeUninit;

const ROUTER_CAPACITY: usize = 64;

/// Wraps a chain of render passes and provides an event router for handling events.
/// The `E` type parameter represents the event type that can be dispatched
/// to the passes. The `C` type parameter is a compile-time heterogeneous
/// list of render passes that implements `ChainExecute`.
pub struct RenderPipeline<C, E>
where
    E: PassEventTrait,
    C: RenderChain<E>,
{
    chain: C,
    event_router: [PassEventTarget<E>; ROUTER_CAPACITY],
}

impl<C, E> RenderPipeline<C, E>
where
    E: PassEventTrait,
    C: RenderChain<E>,
{
    pub fn new(chain: C) -> Self {
        let l = chain.length();
        if l > MAX_RENDER_PASSES {
            panic!(
                "Render chain length exceeds maximum allowed passes: {} > {}",
                l, MAX_RENDER_PASSES
            );
        }

        let targets = chain.get_targets();
        if targets.len() > ROUTER_CAPACITY {
            panic!(
                "Render pass targets exceed router capacity: {} > {}",
                targets.len(),
                ROUTER_CAPACITY
            );
        }

        // Create an uninitialized array of `MaybeUninit` to not require
        // `Default` or `Clone` on `PassEventTarget`.
        let mut event_router: [MaybeUninit<PassEventTarget<E>>; ROUTER_CAPACITY] =
            [const { MaybeUninit::uninit() }; ROUTER_CAPACITY];
        for target in targets {
            let id = target.get_id();
            event_router[id.as_usize()].write(target);
        }

        RenderPipeline {
            chain,

            // Everything is initialized. Transmute the array to the
            // initialized type.
            event_router: unsafe {
                std::mem::transmute::<
                    [MaybeUninit<PassEventTarget<E>>; ROUTER_CAPACITY],
                    [PassEventTarget<E>; ROUTER_CAPACITY],
                >(event_router)
            },
        }
    }

    pub(crate) fn dispatch(&self, e: RenderPassEvent<E>) {
        let index = e.get_target_id().as_usize();

        #[cfg(debug_assertions)]
        if index >= ROUTER_CAPACITY {
            panic!("RenderPassEvent target ID out of bounds: {}", index);
        }

        #[cfg(debug_assertions)]
        if self.event_router[index].get_id().as_usize() == 0 {
            panic!("RenderPassEvent target ID is not registered: {}", index);
        }

        // Dispatch the event to the appropriate target.
        self.event_router[index].dispatch(e.event());
    }

    pub(crate) fn get_names(&self) -> Vec<&str> {
        // Collect the names of all render passes in the chain.
        self.chain.get_names()
    }

    #[inline(always)]
    pub(crate) fn execute(&mut self, ctx: &mut ChainExecuteCtx<E>) -> RenderResult {
        // Execute the chain of render passes.
        self.chain.execute(0, ctx)
    }
}
