#![feature(test)]
#![feature(maybe_uninit_slice)]

pub mod assets;
pub mod backend;
mod cpal;
pub mod dsp;
pub mod entities;
pub mod player;
mod sample;

pub type SamplesCount = usize;
pub type SampleRate = usize;
pub type ChannelsCount = usize;

/// Hardcoded sample type of the data transferred between the audio
/// processing thread and the audio device.
/// All the processing is done in f32 format and cannot be changed.
pub type SampleType = f32;

/// Hardcoded number of channels.
/// This is a stereo sound device, so it has 2 channels.
const CHANNELS_COUNT: ChannelsCount = 2;

/// Hardcoded size of the audio block.
/// This is the number of samples processed in one block.
const BLOCK_SIZE: SamplesCount = 512;
