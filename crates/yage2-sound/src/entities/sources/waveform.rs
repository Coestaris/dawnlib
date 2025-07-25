use crate::entities::events::{Event, EventTarget, EventTargetId};
use crate::entities::{BlockInfo, Source};
use crate::sample::PlanarBlock;

#[derive(Debug, Clone, PartialEq)]
pub enum WaveformType {
    Disabled,
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WaveformSourceEvent {
    SetWaveformType(WaveformType),
    SetFrequency(f32),
    SetPhase(f32),
}

/// Allows generating audio samples on the fly,
/// for example, a sine wave generator.
pub struct WaveformSource {
    id: EventTargetId,
    cached: bool,
    waveform_type: WaveformType,
    frequency: f32,
    phase: f32,
    output: PlanarBlock<f32>,
}

fn dispatch_waveform(ptr: *mut u8, event: &Event) {
    let waveform: &mut WaveformSource = unsafe { &mut *(ptr as *mut WaveformSource) };
    waveform.dispatch(event);
}

impl WaveformSource {
    pub fn new(
        waveform_type: Option<WaveformType>,
        frequency: Option<f32>,
        phase: Option<f32>,
    ) -> Self {
        WaveformSource {
            waveform_type: waveform_type.unwrap_or(WaveformType::Disabled),
            frequency: frequency.unwrap_or(0.0),
            phase: phase.unwrap_or(0.0),

            id: EventTargetId::new(),
            cached: false,
            output: PlanarBlock::default(),
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
    use crate::sample::PlanarBlock;
    use crate::BLOCK_SIZE;

    pub(crate) fn generate_sine(frequency: f32, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        fn sine(frequency: f32, time: f32) -> f32 {
            (2.0 * std::f32::consts::PI * frequency * time).sin() * 0.1
        }

        // TODO: Implement SIMD optimization for sine wave generation
        for i in 0..BLOCK_SIZE {
            let time = info.time(i);
            let value = sine(frequency, time);
            for channel in 0..output.samples.len() {
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
            for channel in 0..output.samples.len() {
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
            for channel in 0..output.samples.len() {
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
            for channel in 0..output.samples.len() {
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
            Event::Waveform(WaveformSourceEvent::SetFrequency(frequency)) => {
                self.frequency = *frequency;
                self.cached = false;
            }
            Event::Waveform(WaveformSourceEvent::SetPhase(phase)) => {
                self.phase = *phase; // Phase is not used in this implementation, but can be added later
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
            WaveformType::Sine => dsp::generate_sine(self.frequency, &mut self.output, info),
            WaveformType::Square => dsp::generate_square(self.frequency, &mut self.output, info),
            WaveformType::Triangle => {
                dsp::generate_triangle(self.frequency, &mut self.output, info)
            }
            WaveformType::Sawtooth => {
                dsp::generate_sawtooth(self.frequency, &mut self.output, info)
            }
        }

        self.cached = true;
        &self.output
    }
}
