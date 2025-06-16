use std::sync::Arc;
use crate::engine::input::InputManager;

pub struct ApplicationCtx {
    pub(crate) input_manager: InputManager,
}

impl ApplicationCtx {
    fn close(&mut self) {;
        // Logic to close the application context, e.g., clean up resources
        // This could include closing the input manager, graphics context, etc.
        // For now, we will just print a message.
        println!("Closing Application Context");
    }
    
    fn add_object<T: 'static + Send + Sync>(&mut self, object: T) {
        // Logic to add an object to the application context
        // This could involve storing the object in a collection for later use
        // For now, we will just print a message.
        println!("Adding object: {:?}", std::any::type_name::<T>());
    }
}


