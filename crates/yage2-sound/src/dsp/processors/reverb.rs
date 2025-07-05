use crate::control::{new_control, ControlReceiver, Controller};
use crate::dsp::{BlockInfo, EventDispatcher, Processor, ProcessorType};
use crate::sample::{PlanarBlock, LEFT_CHANNEL, RIGHT_CHANNEL};
use crate::{BLOCK_SIZE, CHANNELS_COUNT};

const LINES_COUNT: usize = 2;

const DEFAULT_DELAY_SIZE: usize = 20000; // Around of half a second at 44.1 kHz
const DEFAULT_REVERB_FACTOR: f32 = 0.5; // Default reverb factor
const DEFAULT_MIX_LEVEL: f32 = 0.5; // Default mix level

pub enum LineReverbMessage {
    SetLineSize(usize, usize), // Set the size of the delay line
    SetLineFade(usize, f32),   // Set the fade factor for the delay line
    SetWetLevel(usize, f32),   // Set the dry level for the delay line
}

struct DelayLine {
    samples: [Vec<f32>; CHANNELS_COUNT], // Delay line buffer
    index: usize,                        // Current index in the delay line
    size: usize,                         // Size of the delay line
    fade: f32,
    mix_level: f32,
}

impl Default for DelayLine {
    fn default() -> Self {
        Self {
            samples: [
                vec![0.0; DEFAULT_DELAY_SIZE], // Left channel
                vec![0.0; DEFAULT_DELAY_SIZE], // Right channel
            ],
            index: 0,
            size: DEFAULT_DELAY_SIZE,
            fade: DEFAULT_REVERB_FACTOR,
            mix_level: DEFAULT_MIX_LEVEL,
        }
    }
}

pub struct Reverb {
    delay_lines: [DelayLine; LINES_COUNT],
    receiver: ControlReceiver<LineReverbMessage>,
}

impl Reverb {
    pub fn new() -> (ProcessorType, Controller<LineReverbMessage>) {
        let (controller, receiver) = new_control();
        let reverb = Self {
            receiver,
            delay_lines: [DelayLine::default(), DelayLine::default()],
        };
        (ProcessorType::Reverb(reverb), controller)
    }
}

impl EventDispatcher for Reverb {
    fn dispatch_events(&mut self) {
        while let Some(message) = self.receiver.receive() {
            match message {
                LineReverbMessage::SetLineSize(line, size) => {
                    if line < LINES_COUNT {
                        self.delay_lines[line].size = size.max(1); // Ensure size is at least 1
                        self.delay_lines[line].samples = [vec![0.0; size], vec![0.0; size]];
                        self.delay_lines[line].index = 0; // Reset index
                    }
                }
                LineReverbMessage::SetLineFade(line, fade) => {
                    if line < LINES_COUNT {
                        self.delay_lines[line].fade = fade.clamp(0.0, 1.0); // Limit to [0.0, 1.0]
                    }
                }
                LineReverbMessage::SetWetLevel(line, mix) => {
                    if line < LINES_COUNT {
                        self.delay_lines[line].mix_level = mix.clamp(0.0, 1.0); // Limit to [0.0, 1.0]
                    }
                }
            }
        }
    }
}

fn add_line(input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, delay_line: &mut DelayLine) {
    // TODO: Implement some batch processing logic
    for i in 0..BLOCK_SIZE {
        // Read from the delay line
        let delayed_left = delay_line.samples[LEFT_CHANNEL][delay_line.index];
        let delayed_right = delay_line.samples[RIGHT_CHANNEL][delay_line.index];

        let left = input.samples[LEFT_CHANNEL][i];
        let right = input.samples[RIGHT_CHANNEL][i];

        // Write the current sample to the delay line
        delay_line.samples[LEFT_CHANNEL][delay_line.index] = left + delayed_left * delay_line.fade;
        delay_line.samples[RIGHT_CHANNEL][delay_line.index] =
            right + delayed_right * delay_line.fade;

        // Update the index for the next sample
        delay_line.index = (delay_line.index + 1) % delay_line.size;

        // Output the processed sample
        output.samples[LEFT_CHANNEL][i] +=
            left * (1.0 - delay_line.mix_level) + delayed_left * delay_line.mix_level;
        output.samples[RIGHT_CHANNEL][i] +=
            right * (1.0 - delay_line.mix_level) + delayed_right * delay_line.mix_level;
    }
}

impl Processor for Reverb {
    fn process(&mut self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, _: &BlockInfo) {
        // Process each delay line
        for delay_line in &mut self.delay_lines {
            add_line(input, output, delay_line);
        }
        
        // Ensure output is clamped to prevent overflow
        for channel in 0..CHANNELS_COUNT {
            for sample in &mut output.samples[channel] {
                *sample /= LINES_COUNT as f32;
            }
        }
    }
}
