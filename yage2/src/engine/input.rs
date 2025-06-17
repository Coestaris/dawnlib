use crate::core::time::TickCounter;
pub(crate) use crate::engine::event::{Event, KeyCode, MouseButton};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

pub struct InputManager {
    receiver: Receiver<Event>,
    eps: Arc<TickCounter>,

    /* Cache for the current input state.
     * This is a simple representation; in a real application, you might want to
     * use more sophisticated structures or libraries for input handling. */
    buttons_state: [bool; 3], // Assuming 3 mouse buttons: left, right, middle
    keys_state: [bool; 256],  // Assuming 256 keys for simplicity
    mouse_position: (f32, f32), // Current mouse position
}

impl InputManager {
    pub(crate) fn new(receiver: Receiver<Event>, eps: Arc<TickCounter>) -> Self {
        InputManager {
            receiver,
            eps,
            buttons_state: [false; 3], // Initialize all mouse buttons to not pressed
            keys_state: [false; 256],  // Initialize all keys to not pressed
            mouse_position: (0.0, 0.0), // Initialize mouse position to (0, 0)
        }
    }

    pub(crate) fn poll_events(&mut self) -> Vec<Event> {
        let mut events: Vec<Event> = Vec::new();

        /* Poll the receiver for new events and return them.
         * This method can be called to retrieve events without blocking. */
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);

            #[cfg(debug_assertions)]
            self.eps.tick();
        }

        events
    }

    pub(crate) fn on_event(&self, event: &Event) {
        match event {
            Event::KeyPress(key) => {}
            Event::KeyRelease(key) => {}
            Event::MouseMove { x, y } => {}
            Event::MouseButtonPress(button) => {}
            Event::MouseButtonRelease(button) => {}
            _ => {}
        }
    }

    /* Handy methods to retrieve input state */
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool {
        // self.keys_state[key as usize]
        false
    }

    pub fn mouse_button_pressed(&self, button: MouseButton) -> bool {
        // self.buttons_state[button as usize]
        false
    }
}
