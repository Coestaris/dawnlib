use crate::dsp::{BlockInfo, EventDispatcher, Processor, ProcessorType};
use crate::control::{new_control, ControlReceiver, Controller};
use crate::sample::PlanarBlock;

pub enum HPFMessage {
    SetCutoff(f32),    // Set the cutoff frequency
    SetResonance(f32), // Set the resonance amount
}

pub struct HPF {
    pub cutoff: f32,    // Cutoff frequency
    pub resonance: f32, // Resonance amount

    receiver: ControlReceiver<HPFMessage>,
}

impl HPF {
    pub fn new() -> (ProcessorType, Controller<HPFMessage>) {
        let (controller, receiver) = new_control();
        let hpf = Self {
            cutoff: 0.5,    // Default value
            resonance: 0.5, // Default value
            receiver,
        };
        (ProcessorType::HPF(hpf), controller)
    }
}

impl EventDispatcher for HPF {
    fn dispatch_events(&mut self) {
        // Process messages from the channel
        while let Some(message) = self.receiver.receive() {
            match message {
                HPFMessage::SetCutoff(cutoff) => {
                    self.cutoff = cutoff.clamp(0.0, 1.0); // Limit to [0.0, 1.0]
                }
                HPFMessage::SetResonance(resonance) => {
                    self.resonance = resonance.clamp(0.0, 1.0); // Limit to [0.0, 1.0]
                }
            }
        }
    }
}

impl Processor for HPF {
    fn process(&mut self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, _: &BlockInfo) {
        // Bypass for now, just copy input to output
        output.copy_from(input);
    }
}
