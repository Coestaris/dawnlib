use crate::control::{new_control, ControlReceiver, Controller};
use crate::dsp::{BlockInfo, EventDispatcher, Processor};
use crate::sample::PlanarBlock;
use crate::{BLOCK_SIZE, CHANNELS_COUNT};

const DEFAULT_DELAY_SIZE: usize = 20000; // Around of half a second at 44.1 kHz

pub enum ReverbMessage {
    SetDelaySize(usize),  // Set the size of the delay line
    SetReverbFactor(f32), // Set the reverb factor
}

struct DelayLine {
    samples: Vec<f32>, // Delay line buffer
    index: usize,      // Current index in the delay line
}

impl Default for DelayLine {
    fn default() -> Self {
        Self {
            samples: vec![0.0; DEFAULT_DELAY_SIZE],
            index: 0,
        }
    }
}

pub struct Reverb {
    delay_size: usize,  // Size of the delay line (in samples)
    reverb_factor: f32, // Factor to apply to the reverb effect

    delay_lines: [DelayLine; CHANNELS_COUNT as usize], // Delay line buffer

    receiver: ControlReceiver<ReverbMessage>,
}

impl Reverb {
    pub fn new() -> (Self, Controller<ReverbMessage>) {
        let (controller, receiver) = new_control();
        let reverb = Self {
            delay_size: DEFAULT_DELAY_SIZE, // 1 second delay at 44.1 kHz
            reverb_factor: 0.5,             // Default reverb factor
            receiver,
            delay_lines: [
                DelayLine::default(), // Left channel
                DelayLine::default(), // Right channel
            ],
        };
        (reverb, controller)
    }
}

impl EventDispatcher for Reverb {
    fn dispatch_events(&mut self) {
        while let Some(message) = self.receiver.receive() {
            match message {
                ReverbMessage::SetDelaySize(size) => {
                    if size > 0 {
                        self.delay_size = size;
                        for delay_line in &mut self.delay_lines {
                            delay_line.samples.resize(size, 0.0);
                            delay_line.index = 0; // Reset index
                        }
                    }
                }
                ReverbMessage::SetReverbFactor(factor) => {
                    self.reverb_factor = factor.clamp(0.0, 1.0); // Limit to [0.0, 1.0]
                }
            }
        }
    }
}

fn process_channel(
    input: &[f32],
    output: &mut [f32],
    delay_line: &mut DelayLine,
    reverb_factor: f32,
) {
    // TODO: Implement some batch processing logic
    for (i, &sample) in input.iter().enumerate() {
        // Read from the delay line
        let delayed_sample = delay_line.samples[delay_line.index];

        // Write the current sample to the delay line
        delay_line.samples[delay_line.index] = sample + delayed_sample * reverb_factor;

        // Update the index for the next sample
        delay_line.index = (delay_line.index + 1) % delay_line.samples.len();

        // Output the processed sample
        output[i] = delayed_sample;
    }
}

impl Processor for Reverb {
    fn process(&mut self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, _: &BlockInfo) {
        for channel in 0..CHANNELS_COUNT as usize {
            let input_channel = &input.samples[channel];
            let output_channel = &mut output.samples[channel];

            // Process each channel with the delay line
            process_channel(
                input_channel,
                output_channel,
                &mut self.delay_lines[channel as usize],
                self.reverb_factor,
            );
        }
    }
}
