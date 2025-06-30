use crate::control::{new_control, ControlReceiver, Controller};
use crate::dsp::{BlockInfo, EventDispatcher, Processor};
use crate::sample::PlanarBlock;

pub enum ReverbMessage {
    SetRoomSize(f32), // Set the size of the reverb room
    SetDamping(f32),  // Set the damping factor
}

pub struct Reverb {
    pub room_size: f32, // Size of the reverb room
    pub damping: f32,   // Damping factor

    receiver: ControlReceiver<ReverbMessage>,
}

impl Reverb {
    pub fn new() -> (Self, Controller<ReverbMessage>) {
        let (controller, receiver) = new_control();
        let reverb = Self {
            room_size: 0.5, // Default value
            damping: 0.5,   // Default value
            receiver,
        };
        (reverb, controller)
    }
}

impl EventDispatcher for Reverb {
    fn dispatch_events(&mut self) {
        while let Some(message) = self.receiver.receive() {
            match message {
                ReverbMessage::SetRoomSize(size) => {
                    self.room_size = size.clamp(0.0, 1.0); // Limit to [0.0, 1.0]
                }
                ReverbMessage::SetDamping(damping) => {
                    self.damping = damping.clamp(0.0, 1.0); // Limit to [0.0, 1.0]
                }
            }
        }
    }
}

impl Processor for Reverb {
    fn process(&self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        // Bypass for now, just copy input to output
        output.copy_from(input);
    }
}
