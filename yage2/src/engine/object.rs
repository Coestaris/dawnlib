use crate::engine::event::{Event, EventMask};
use crate::engine::input::InputManager;
use std::sync::{Arc, Mutex};

/// Represents a renderable object in the engine.
#[derive(Debug, Clone, Default)]
pub struct Renderable {
    // Placeholder for renderable data.
    pub sample_data: f32,
    pub sample_data2: usize,
}

/// Context passed to the `Object` during event dispatching.
/// Contains references to the input manager and other necessary components.
pub struct ObjectCtx<'a> {
    pub input_manager: &'a InputManager,
}

/// Type alias for an object pointer, which is a thread-safe reference counted pointer.
pub type ObjectPtr = Arc<Mutex<dyn Object + Send + Sync>>;

pub enum DispatchAction {
    /// No action to be taken.
    Empty,

    /// Request to kill the object.
    Die,

    /// Request to kill an object.
    KillObject(ObjectPtr),

    /// Request to kill multiple objects.
    KillObjects(Vec<ObjectPtr>),

    /// Request to spawn multiple objects.
    SpawnObjects(Vec<ObjectPtr>),

    /// Request to spawn a new object.
    SpawnObject(ObjectPtr),

    /// Update of the renderable representing the object.
    /// This is used to update the state of the object
    UpdateRenderable(Renderable),

    /// Request to delete the renderable.
    DeleteRenderable,

    /// Request to quit the application.
    QuitApplication,
}

pub trait Object: Send + Sync + 'static {
    /// Called to initialize the object.
    /// This is where you can set up event listeners or initial state.
    fn event_mask(&self) -> EventMask {
        // The default implementation returns an empty mask.
        // Override this method in derived classes to specify
        // which events the object is interested in.
        EventMask::empty()
    }

    /// Called when a matching event is dispatched
    fn dispatch(&mut self, _: &ObjectCtx, _: &Event) -> DispatchAction {
        // The default implementation does nothing.
        // Override this method in derived classes to handle
        // specific events.
        DispatchAction::Empty
    }
}

#[macro_export]
macro_rules! create_object {
        ($expression:expr) => {
          {
            use std::sync::{Arc, Mutex};
          Arc::new(Mutex::new($expression)) as yage2::engine::object::ObjectPtr
          }
        };
        ($object_type:ty, $($args:expr),*) => {
            Arc::new(Mutex::new($object_type::new($($args),*))) as yage2::engine::object::ObjectPtr
        };
    }
