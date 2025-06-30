use crate::dsp::processors::delay::Delay;
use crate::dsp::processors::hpf::HPF;
use crate::dsp::processors::lpf::LPF;
use crate::dsp::processors::reverb::Reverb;
use crate::dsp::sources::clip::ClipSource;
use crate::dsp::sources::group::GroupSource;
use crate::dsp::sources::sampler::SamplerSource;
use crate::dsp::sources::waveform::WaveformSource;
use crate::sample::PlanarBlock;
use std::sync::mpsc::Sender;
use std::sync::Arc;

pub mod bus;
mod math;
pub mod processors;
pub mod sources;

pub(crate) struct BlockInfo {
    pub(crate) sample_index: usize,
    pub(crate) sample_rate: u32,
}

impl BlockInfo {
    fn time(&self, i: usize) -> f32 {
        (self.sample_index as f32 + i as f32) / self.sample_rate as f32
    }
}

pub(crate) trait EventDispatcher {
    fn dispatch_events(&mut self);
}

pub(crate) trait Processor {
    fn process(&self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, info: &BlockInfo);
}

pub enum ProcessorType {
    NoProcessor,
    Delay(Delay),
    Reverb(Reverb),
    LPF(LPF),
    HPF(HPF),
}

impl Default for ProcessorType {
    fn default() -> Self {
        ProcessorType::NoProcessor
    }
}

impl Processor for ProcessorType {
    fn process(&self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        match self {
            ProcessorType::NoProcessor => {
                // No processor, just copy input to output
                output.copy_from(input);
                return;
            }
            ProcessorType::Delay(delay) => delay.process(input, output, info),
            ProcessorType::Reverb(reverb) => reverb.process(input, output, info),
            ProcessorType::LPF(lpf) => lpf.process(input, output, info),
            ProcessorType::HPF(hpf) => hpf.process(input, output, info),
        }
    }
}

impl EventDispatcher for ProcessorType {
    fn dispatch_events(&mut self) {
        match self {
            ProcessorType::NoProcessor => {
                // No events to process
            }
            ProcessorType::Delay(delay) => delay.dispatch_events(),
            ProcessorType::Reverb(reverb) => reverb.dispatch_events(),
            ProcessorType::LPF(lpf) => lpf.dispatch_events(),
            ProcessorType::HPF(hpf) => hpf.dispatch_events(),
        }
    }
}

pub(crate) trait Generator {
    fn generate(&self, output: &mut PlanarBlock<f32>, info: &BlockInfo);
}

pub enum SourceType {
    NoSource,
    Sampler(SamplerSource),
    Clip(ClipSource),
    Waveform(WaveformSource),
    Group(GroupSource),
}

impl Default for SourceType {
    fn default() -> Self {
        SourceType::NoSource
    }
}

impl Generator for SourceType {
    fn generate(&self, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        match self {
            SourceType::NoSource => {
                // No source, do nothing
                return;
            }

            SourceType::Sampler(sampler) => sampler.generate(output, info),
            SourceType::Clip(clip) => clip.generate(output, info),
            SourceType::Waveform(waveform) => waveform.generate(output, info),
            SourceType::Group(group) => group.generate(output, info),
        }
    }
}

impl EventDispatcher for SourceType {
    fn dispatch_events(&mut self) {
        match self {
            SourceType::NoSource => {
                // No events to process
            }
            SourceType::Sampler(sampler) => sampler.dispatch_events(),
            SourceType::Clip(clip) => clip.dispatch_events(),
            SourceType::Waveform(waveform) => waveform.dispatch_events(),
            SourceType::Group(group) => group.dispatch_events(),
        }
    }
}
