#![feature(test)]

pub mod backend;
mod cpal;
pub mod dsp;
mod error;
pub mod manager;
mod resources;
mod sample;
pub mod entities;

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
const BLOCK_SIZE: SamplesCount = 256;
