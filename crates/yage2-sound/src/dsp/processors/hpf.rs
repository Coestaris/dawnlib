use crate::dsp::{BlockInfo, Control, Processor};
use crate::sample::PlanarBlock;

enum HPFMessage {
    SetCutoff(f32),    // Set the cutoff frequency
    SetResonance(f32), // Set the resonance amount
}

pub struct HPF {
    pub cutoff: f32,    // Cutoff frequency
    pub resonance: f32, // Resonance amount

    receiver: std::sync::mpsc::Receiver<HPFMessage>,
}

impl HPF {
    pub fn new() -> (Self, std::sync::mpsc::Sender<HPFMessage>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let hpf = Self {
            cutoff: 0.5,    // Default value
            resonance: 0.5, // Default value
            receiver,
        };
        (hpf, sender)
    }
}

impl Control for HPF {
    fn process_events(&mut self) {
        // Process messages from the channel
        while let Ok(message) = self.receiver.try_recv() {
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
    fn process(&self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        // Bypass for now, just copy input to output
        output.copy_from(input);
    }
}
