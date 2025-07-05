use crate::dsp::{BlockInfo, EventDispatcher, Processor};
use crate::control::{new_control, ControlReceiver, Controller};
use crate::sample::PlanarBlock;

pub enum DelayMessage {
    SetDelay(usize),  // Set the delay in samples
    SetFeedback(f32), // Set the feedback amount
}

pub struct Delay {
    pub delay: usize,  // Delay in samples
    pub feedback: f32, // Feedback amount

    receiver: ControlReceiver<DelayMessage>,
}

impl Delay {
    pub fn new() -> (Self, Controller<DelayMessage>) {
        let (controller, receiver) = new_control();
        let delay = Self {
            delay: 0,      // Default value
            feedback: 0.0, // Default value
            receiver,
        };
        (delay, controller)
    }
}

impl EventDispatcher for Delay {
    fn dispatch_events(&mut self) {
        // Process messages from the channel
        while let Some(message) = self.receiver.receive() {
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
    fn process(&mut self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, _: &BlockInfo) {
        // Bypass for now, just copy input to output
        output.copy_from(input);
    }
}
