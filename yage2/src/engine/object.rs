use crate::engine::app_ctx::ApplicationCtx;
use crate::engine::event::{InputEvent, InputEventKind};

#[derive(Debug, Clone)]
pub struct Renderable {
    /* This struct can be used to represent objects that
     * can be rendered in the game. It can hold information
     * about the object's position, size, and other properties. */
}

pub trait Object {
    /* Called to initialize the object.
     * This is where you can set up event listeners or initial state. */
    fn events_mask(&self, ctx: &ApplicationCtx) -> InputEventKind {
        /* The default implementation returns an empty mask.
         * Override this method in derived classes to specify
         * which events the object is interested in. */
        InputEventKind::empty()
    }

    /* Called when a matching event is dispatched */
    fn on_event(&mut self, ctx: &ApplicationCtx, event: &InputEvent) {
        /* The default implementation does nothing.
         * Override this method in derived classes to handle
         * specific events. */
    }

    /* Called on each tick of the game loop.
     * This is where you can update the object's
     * state based on input or other factors. */
    fn on_tick(&mut self, _: &ApplicationCtx) {
        /* The default implementation does nothing.
         * Override this method in derived classes if needed. */
    }
    
    fn renderable(&self) -> Option<&Renderable> {
        /* The default implementation returns None.
         * Override this method in derived classes to provide
         * a Renderable object if the object can be rendered. */
        None
    }
}
