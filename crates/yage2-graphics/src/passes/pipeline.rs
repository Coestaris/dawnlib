use crate::passes::chain::ChainExecute;
use crate::passes::events::{PassEventTarget, RenderPassEvent};
use crate::passes::result::PassExecuteResult;
use crate::passes::PassExecuteContext;

const ROUTER_CAPACITY: usize = 64;

/// Wraps a chain of render passes and provides an event router for handling events.
/// The `E` type parameter represents the event type that can be dispatched
/// to the passes. The `C` type parameter is a compile-time heterogeneous
/// list of render passes that implements `ChainExecute`.
pub struct RenderPipeline<C, E>
where
    E: Copy + 'static,
    C: ChainExecute<E> + Send + Sync + 'static,
{
    chain: C,
    event_router: [PassEventTarget<E>; ROUTER_CAPACITY],
}

impl<C, E> RenderPipeline<C, E>
where
    E: Copy + 'static,
    C: ChainExecute<E> + Send + Sync + 'static,
{
    pub fn new(chain: C) -> Self {
        let tragets = chain.get_targets();
        let mut event_router = [PassEventTarget::default(); ROUTER_CAPACITY];
        for target in tragets {
            event_router[target.get_id().as_usize()] = target;
        }

        RenderPipeline {
            chain,
            event_router,
        }
    }

    pub(crate) fn dispatch(&self, e: &RenderPassEvent<E>) {
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
        self.event_router[index].dispatch(e.get_event());
    }

    pub(crate) fn execute(&mut self, ctx: &PassExecuteContext) -> PassExecuteResult {
        // Execute the chain of render passes.
        self.chain.execute(ctx)
    }
}
