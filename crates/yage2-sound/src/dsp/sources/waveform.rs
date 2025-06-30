use crate::dsp::{BlockInfo, EventDispatcher, Generator};
use crate::control::{new_control, ControlReceiver, Controller};
use crate::sample::PlanarBlock;
use crate::BLOCK_SIZE;
use log::debug;

pub enum WaveformMessage {
    SetWaveformType(WaveformType),
    SetFrequency(f32),
    SetPhase(f32),
}

pub struct WaveformControllable {
    waveform_type: WaveformType,
    frequency: f32,
    phase: f32,
}

pub enum WaveformType {
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

/// Allows generating audio samples on the fly,
/// for example, a sine wave generator.
pub struct WaveformSource {
    controllable: WaveformControllable,
    receiver: ControlReceiver<WaveformMessage>,
}

impl WaveformSource {
    pub fn new() -> WaveformControllable {
        WaveformControllable {
            waveform_type: WaveformType::Sine,
            frequency: 440.0, // Default to A4 note
            phase: 0.0,
        }
    }
}

impl WaveformControllable {
    pub fn build(self) -> (WaveformSource, Controller<WaveformMessage>) {
        let (controller, receiver) = new_control();
        let source = WaveformSource {
            controllable: self,
            receiver,
        };
        (source, controller)
    }

    pub fn set_waveform_type(mut self, waveform_type: WaveformType) -> Self {
        WaveformControllable::set_waveform_type_inner(&mut self, waveform_type);
        self
    }

    pub fn set_frequency(mut self, frequency: f32) -> Self {
        WaveformControllable::set_frequency_inner(&mut self, frequency);
        self
    }

    pub fn set_phase(mut self, phase: f32) -> Self {
        WaveformControllable::set_phase_inner(&mut self, phase);
        self
    }

    fn set_waveform_type_inner(&mut self, waveform_type: WaveformType) {
        self.waveform_type = waveform_type;
    }

    fn set_frequency_inner(&mut self, frequency: f32) {
        self.frequency = frequency.clamp(5.0, 20000.0); // Limit to audible range
    }

    fn set_phase_inner(&mut self, phase: f32) {
        self.phase = phase; // Phase can be any value
    }
}

impl EventDispatcher for WaveformSource {
    fn dispatch_events(&mut self) {
        while let Some(message) = self.receiver.receive() {
            match message {
                WaveformMessage::SetWaveformType(waveform_type) => {
                    self.controllable.set_waveform_type_inner(waveform_type);
                }
                WaveformMessage::SetFrequency(frequency) => {
                    debug!("Setting frequency to {}", frequency);
                    self.controllable.set_frequency_inner(frequency);
                }
                WaveformMessage::SetPhase(phase) => {
                    self.controllable.set_phase_inner(phase);
                }
            }
        }
    }
}

fn sine_wave(frequency: f32, time: f32) -> f32 {
    (2.0 * std::f32::consts::PI * frequency * time).sin()
}

fn square_wave(frequency: f32, time: f32) -> f32 {
    if (frequency * time) % 1.0 < 0.5 {
        1.0
    } else {
        -1.0
    }
}

fn triangle_wave(frequency: f32, time: f32) -> f32 {
    2.0 * (frequency * time - (frequency * time).floor()) - 1.0
}

fn sawtooth_wave(frequency: f32, time: f32) -> f32 {
    (frequency * time) % 1.0 * 2.0 - 1.0
}

impl Generator for WaveformSource {
    fn generate(&self, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        let data = &self.controllable;
        let frequency = data.frequency;

        for i in 0..BLOCK_SIZE {
            let time = info.time(i);
            let value = match data.waveform_type {
                WaveformType::Sine => sine_wave(frequency, time + data.phase),
                WaveformType::Square => square_wave(frequency, time + data.phase),
                WaveformType::Triangle => triangle_wave(frequency, time + data.phase),
                WaveformType::Sawtooth => sawtooth_wave(frequency, time + data.phase),
            };

            // Fill all channels with the generated sample
            for channel in 0..output.samples.len() {
                output.samples[channel][i] = value;
            }
        }
    }
}
