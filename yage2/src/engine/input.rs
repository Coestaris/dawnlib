use crate::core::time::TickCounter;
pub(crate) use crate::engine::event::{Event, KeyCode, MouseButton};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

pub struct InputManager {
    receiver: Receiver<Event>,
    eps: Arc<TickCounter>,

    // Maps mouse buttons to their pressed state
    buttons_state: HashMap<MouseButton, bool>,
    // Maps keys to their pressed state
    keys_state: HashMap<KeyCode, bool>,
    // Current mouse position
    mouse_position: (f32, f32),
}

impl InputManager {
    pub(crate) fn new(receiver: Receiver<Event>, eps: Arc<TickCounter>) -> Self {
        InputManager {
            receiver,
            eps,
            buttons_state: HashMap::new(),
            keys_state: HashMap::new(),
            mouse_position: (0.0, 0.0), // Initialize mouse position to (0, 0)
        }
    }

    pub(crate) fn poll_events(&mut self) -> Vec<Event> {
        let mut events: Vec<Event> = Vec::new();

        /* Poll the receiver for new events and return them.
         * This method can be called to retrieve events without blocking. */
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }

        #[cfg(debug_assertions)]
        self.eps.tick(events.len() as u32);
        events
    }

    pub(crate) fn on_event(&mut self, event: &Event) {
        match event {
            Event::KeyPress(key) => {
                self.keys_state.insert(*key, true);
            }
            Event::KeyRelease(key) => {
                self.keys_state.insert(*key, false);
            }
            Event::MouseMove { x, y } => {
                self.mouse_position = (*x, *y);
            }
            Event::MouseButtonPress(button) => {
                self.buttons_state.insert(*button, true);
            }
            Event::MouseButtonRelease(button) => {
                self.buttons_state.insert(*button, false);
            }
            _ => {}
        }
    }

    /* Handy methods to retrieve input state */
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool {
        match self.keys_state.get(&key) {
            Some(&pressed) => pressed,
            None => false,
        }
    }

    pub fn mouse_button_pressed(&self, button: MouseButton) -> bool {
        match self.buttons_state.get(&button) {
            Some(&pressed) => pressed,
            None => false,
        }
    }
}
