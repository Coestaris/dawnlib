use crate::entities::events::{Event, EventTarget, EventTargetId};
use crate::entities::{BlockInfo, Source};
use crate::sample::PlanarBlock;
use tinyrand::Wyrand;

#[derive(Debug, Clone, PartialEq)]
pub enum WaveformType {
    Disabled,
    WhiteNoise,
    Sine(f32),
    Square(f32),
    Triangle(f32),
    Sawtooth(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum WaveformSourceEvent {
    SetWaveformType(WaveformType),
    SetAttack { attack_ms: f32, sample_rate: f32 },
    SetRelease { release_ms: f32, sample_rate: f32 },
}

/// Allows generating audio samples on the fly,
/// for example, a sine wave generator.
pub struct WaveformSource {
    id: EventTargetId,
    cached: bool,
    waveform_type: WaveformType,
    attack: f32,  // In samples
    release: f32, // In samples
    rng: Wyrand,
    output: PlanarBlock<f32>,
}

fn dispatch_waveform(ptr: *mut u8, event: &Event) {
    let waveform: &mut WaveformSource = unsafe { &mut *(ptr as *mut WaveformSource) };
    waveform.dispatch(event);
}

impl WaveformSource {
    pub fn new(waveform_type: Option<WaveformType>) -> Self {
        WaveformSource {
            waveform_type: waveform_type.unwrap_or(WaveformType::Disabled),
            id: EventTargetId::new(),
            cached: false,
            output: PlanarBlock::default(),
            rng: Wyrand::default(),
            attack: 0.0,
            release: 0.0,
        }
    }

    pub fn get_id(&self) -> EventTargetId {
        self.id
    }

    fn create_event_target(&self) -> EventTarget {
        EventTarget::new(dispatch_waveform, self.id, self)
    }
}

mod dsp {
    use crate::entities::BlockInfo;
    use crate::sample::{PlanarBlock, LEFT_CHANNEL, RIGHT_CHANNEL};
    use crate::{BLOCK_SIZE, CHANNELS_COUNT};
    use tinyrand::Rand;

    pub(crate) fn generate_white_noise(
        rng: &mut tinyrand::Wyrand,
        output: &mut PlanarBlock<f32>,
        _info: &BlockInfo,
    ) {
        // Generate white noise by filling the output with random values
        for i in 0..BLOCK_SIZE {
            for channel in 0..CHANNELS_COUNT {
                output.samples[channel][i] = rng.next_u32() as f32 / u32::MAX as f32 * 2.0 - 1.0;
            }
        }
    }

    pub(crate) fn generate_sine(frequency: f32, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        fn sine(frequency: f32, time: f32) -> f32 {
            (2.0 * std::f32::consts::PI * frequency * time).sin()
        }

        // TODO: Implement SIMD optimization for sine wave generation
        for i in 0..BLOCK_SIZE {
            let time = info.time(i);
            let value = sine(frequency, time);
            for channel in 0..CHANNELS_COUNT {
                output.samples[channel][i] = value;
            }
        }
    }

    pub(crate) fn generate_square(frequency: f32, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        fn square(frequency: f32, time: f32) -> f32 {
            if (frequency * time) % 1.0 < 0.5 {
                1.0
            } else {
                -1.0
            }
        }

        // TODO: Implement SIMD optimization for square wave generation
        for i in 0..BLOCK_SIZE {
            let time = info.time(i);
            let value = square(frequency, time);
            for channel in 0..CHANNELS_COUNT {
                output.samples[channel][i] = value;
            }
        }
    }

    pub(crate) fn generate_triangle(
        frequency: f32,
        output: &mut PlanarBlock<f32>,
        info: &BlockInfo,
    ) {
        fn triangle(frequency: f32, time: f32) -> f32 {
            2.0 * (frequency * time - (frequency * time).floor()) - 1.0
        }

        // TODO: Implement SIMD optimization for triangle wave generation
        for i in 0..BLOCK_SIZE {
            let time = info.time(i);
            let value = triangle(frequency, time);
            for channel in 0..CHANNELS_COUNT {
                output.samples[channel][i] = value;
            }
        }
    }

    pub(crate) fn generate_sawtooth(
        frequency: f32,
        output: &mut PlanarBlock<f32>,
        info: &BlockInfo,
    ) {
        fn sawtooth(frequency: f32, time: f32) -> f32 {
            (frequency * time) % 1.0 * 2.0 - 1.0
        }

        for i in 0..BLOCK_SIZE {
            let time = info.time(i);
            let value = sawtooth(frequency, time);
            for channel in 0..CHANNELS_COUNT {
                output.samples[channel][i] = value;
            }
        }
    }
}

impl Source for WaveformSource {
    fn get_targets(&self) -> Vec<EventTarget> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &Event) {
        match event {
            Event::Waveform(WaveformSourceEvent::SetWaveformType(waveform_type)) => {
                self.waveform_type = waveform_type.clone();
                self.cached = false;
            }
            Event::Waveform(WaveformSourceEvent::SetAttack {
                attack_ms,
                sample_rate,
            }) => {
                self.attack = *attack_ms / 1000.0 * sample_rate;
                self.cached = false;
            }
            Event::Waveform(WaveformSourceEvent::SetRelease {
                release_ms,
                sample_rate,
            }) => {
                self.release = *release_ms / 1000.0 * sample_rate;
                self.cached = false;
            }
            _ => {}
        }
    }

    #[inline(always)]
    fn frame_start(&mut self) {
        self.cached = false;
    }

    #[inline(always)]
    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32> {
        if self.cached {
            return &self.output;
        }

        match self.waveform_type {
            WaveformType::Disabled => self.output.silence(),
            WaveformType::WhiteNoise => {
                dsp::generate_white_noise(&mut self.rng, &mut self.output, info)
            }
            WaveformType::Sine(freq) => dsp::generate_sine(freq, &mut self.output, info),
            WaveformType::Square(freq) => dsp::generate_square(freq, &mut self.output, info),
            WaveformType::Triangle(freq) => dsp::generate_triangle(freq, &mut self.output, info),
            WaveformType::Sawtooth(freq) => dsp::generate_sawtooth(freq, &mut self.output, info),
        }

        self.cached = true;
        &self.output
    }
}
