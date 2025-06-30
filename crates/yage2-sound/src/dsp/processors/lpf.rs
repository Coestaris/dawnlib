use crate::dsp::{BlockInfo, Control, Processor};
use crate::sample::PlanarBlock;

enum LPFMessage {
    SetCutoff(f32),    // Set the cutoff frequency
    SetResonance(f32), // Set the resonance amount
}

pub struct LPF {
    pub cutoff: f32,    // Cutoff frequency
    pub resonance: f32, // Resonance amount

    receiver: std::sync::mpsc::Receiver<LPFMessage>,
}

impl LPF {
    pub fn new() -> (Self, std::sync::mpsc::Sender<LPFMessage>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let lpf = Self {
            cutoff: 0.5,    // Default value
            resonance: 0.5, // Default value
            receiver,
        };
        (lpf, sender)
    }
}

impl Control for LPF {
    fn process_events(&mut self) {
        // Process messages from the channel
        while let Ok(message) = self.receiver.try_recv() {
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
    fn process(&self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        // Bypass for now, just copy input to output
        output.copy_from(input);
    }
}
