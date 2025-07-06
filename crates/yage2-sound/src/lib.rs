// #![feature(test)]

pub mod backend;
pub mod control;
mod cpal;
pub mod dsp;
mod error;
pub mod manager;
mod resources;
mod ringbuf;
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

/// Size of the audio device (hardware) buffer.
/// The bigger buffer, the more latency there is, but less CPU usage.
/// The Unit is an interleaved sample.
const DEVICE_BUFFER_SIZE: SamplesCount = 1024;

/// Size of the ring buffer used for transferring audio data from the audio
/// processing thread to the audio device. The bigger buffer, the more latency
/// there is, but the less likely it is to drop samples.
/// Size should be bigger or equal to `DEVICE_BUFFER_SIZE`.
/// The Unit is an interleaved sample.
const RING_BUFFER_SIZE: SamplesCount = DEVICE_BUFFER_SIZE * 2;

/// Minimal operation block size for audio processing.
/// The Unit is an interleaved sample.
const BLOCK_SIZE: SamplesCount = 256;
