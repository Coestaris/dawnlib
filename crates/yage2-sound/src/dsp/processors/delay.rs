use crate::dsp::{BlockInfo, Control, Processor};
use crate::sample::PlanarBlock;

enum DelayMessage {
    SetDelay(usize),  // Set the delay in samples
    SetFeedback(f32), // Set the feedback amount
}

pub struct Delay {
    pub delay: usize,  // Delay in samples
    pub feedback: f32, // Feedback amount

    receiver: std::sync::mpsc::Receiver<DelayMessage>,
}

impl Delay {
    pub fn new() -> (Self, std::sync::mpsc::Sender<DelayMessage>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let delay = Self {
            delay: 0,      // Default value
            feedback: 0.0, // Default value
            receiver,
        };
        (delay, sender)
    }
}

impl Control for Delay {
    fn process_events(&mut self) {
        // Process messages from the channel
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                DelayMessage::SetDelay(samples) => {
                    self.delay = samples.max(0); // Ensure non-negative delay
                }
                DelayMessage::SetFeedback(feedback) => {
                    self.feedback = feedback.clamp(0.0, 1.0); // Limit to [0.0, 1.0]
                }
            }
        }
    }
}

impl Processor for Delay {
    fn process(&self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        // Bypass for now, just copy input to output
        output.copy_from(input);
    }
}
