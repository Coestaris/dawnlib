use crate::core::time::TickCounter;
use crate::engine::app_ctx::ApplicationCtx;
pub(crate) use crate::engine::event::{InputEvent, KeyCode, MouseButton};
use crate::engine::object::Object;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver};

pub struct InputManager {
    receiver: Receiver<InputEvent>,
    eps: Arc<TickCounter>,

    /* Cache for the current input state.
     * This is a simple representation; in a real application, you might want to
     * use more sophisticated structures or libraries for input handling. */
    buttons_state: [bool; 3], // Assuming 3 mouse buttons: left, right, middle
    keys_state: [bool; 256],  // Assuming 256 keys for simplicity
    mouse_position: (f32, f32), // Current mouse position
    events: Vec<InputEvent>,  // Store events for processing
}

impl InputManager {
    pub(crate) fn new(receiver: Receiver<InputEvent>, eps: Arc<TickCounter>) -> Self {
        InputManager {
            receiver,
            eps,
            buttons_state: [false; 3], // Initialize all mouse buttons to not pressed
            keys_state: [false; 256],  // Initialize all keys to not pressed
            mouse_position: (0.0, 0.0), // Initialize mouse position to (0, 0)
            events: Vec::new(),        // Initialize the event storage
        }
    }

    pub(crate) fn poll_events(&mut self) {
        self.events.clear();

        /* Poll the receiver for new events and return them.
         * This method can be called to retrieve events without blocking. */
        while let Ok(event) = self.receiver.try_recv() {
            self.events.push(event);

            #[cfg(debug_assertions)]
            self.eps.tick();
        }
    }

    pub(crate) fn dispatch_events(
        &self,
        ctx: &ApplicationCtx,
        game_objects: &mut [Box<dyn Object + Send + Sync>],
    ) {
        /* Get all input events from the receiver and process them.  */
        for event in self.events.iter() {
            /* Dispatch the event to all game objects
             * that are interested in it.
             * TODO: Iterate over game objects in a more efficient way */
            for game_object in game_objects.iter_mut() {
                if game_object.events_mask(ctx).contains(event.kind()) {
                    game_object.on_event(ctx, &event);
                }
            }
        }
    }

    /* Update the input state based on the event */
    pub(crate) fn update(&mut self) {
        for event in self.events.iter() {
            match event {
                InputEvent::KeyPress(key) => {}
                InputEvent::KeyRelease(key) => {}
                InputEvent::MouseMove { x, y } => {}
                InputEvent::MouseButtonPress(button) => {}
                InputEvent::MouseButtonRelease(button) => {}
                _ => {}
            }
        }
    }

    /* Handy methods to retrieve input state */
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.keys_state[key as usize]
    }

    pub fn mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.buttons_state[button as usize]
    }
}
