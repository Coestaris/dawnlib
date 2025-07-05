use crate::control::{new_control, ControlReceiver, Controller};
use crate::dsp::{BlockInfo, EventDispatcher, Processor, ProcessorType};
use crate::sample::PlanarBlock;

pub enum LPFMessage {
    SetCutoff(f32),    // Set the cutoff frequency
    SetResonance(f32), // Set the resonance amount
}

pub struct LPF {
    pub cutoff: f32,    // Cutoff frequency
    pub resonance: f32, // Resonance amount

    receiver: ControlReceiver<LPFMessage>,
}

impl LPF {
    pub fn new() -> (ProcessorType, Controller<LPFMessage>) {
        let (controller, receiver) = new_control();
        let lpf = Self {
            cutoff: 0.5,    // Default value
            resonance: 0.5, // Default value
            receiver,
        };
        (ProcessorType::LPF(lpf), controller)
    }
}

impl EventDispatcher for LPF {
    fn dispatch_events(&mut self) {
        // Process messages from the channel
        while let Some(message) = self.receiver.receive() {
            match message {
                LPFMessage::SetCutoff(cutoff) => {
                    self.cutoff = cutoff.clamp(0.0, 1.0); // Limit to [0.0, 1.0]
                }
                LPFMessage::SetResonance(resonance) => {
                    self.resonance = resonance.clamp(0.0, 1.0); // Limit to [0.0, 1.0]
                }
            }
        }
    }
}

impl Processor for LPF {
    fn process(&mut self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, _: &BlockInfo) {
        // Bypass for now, just copy input to output
        output.copy_from(input);
    }
}
