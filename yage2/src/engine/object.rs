use crate::engine::event::{Event, EventKind};
use crate::engine::input::InputManager;

#[derive(Debug, Clone)]
pub struct Renderable {
    /* This struct can be used to represent objects that
     * can be rendered in the game. It can hold information
     * about the object's position, size, and other properties. */
}

pub struct ObjectCtx<'a> {
    pub input_manager: &'a InputManager,
}

pub enum ObjectEvent {
    Die,
    SpawnObjects(Vec<Box<dyn Object + Send + Sync>>),
    SpawnObject(Box<dyn Object + Send + Sync>),
    QuitApplication,
}

pub trait Object {
    /* Called to initialize the object.
     * This is where you can set up event listeners or initial state. */
    fn events_mask(&self, ctx: &ObjectCtx) -> EventKind {
        /* The default implementation returns an empty mask.
         * Override this method in derived classes to specify
         * which events the object is interested in. */
        EventKind::empty()
    }

    /* Called when a matching event is dispatched */
    fn on_event(&mut self, ctx: &ObjectCtx, event: &Event) -> Option<ObjectEvent> {
        /* The default implementation does nothing.
         * Override this method in derived classes to handle
         * specific events. */
        None
    }

    fn renderable(&self) -> Option<&Renderable> {
        /* The default implementation returns None.
         * Override this method in derived classes to provide
         * a Renderable object if the object can be rendered. */
        None
    }
}
