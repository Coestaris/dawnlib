use crate::sample::{MappedInterleavedBuffer, Sample};

pub mod backend_impl {
    pub type BackendSpecificConfig = crate::cpal::DeviceConfig;
    pub(crate) type AudioBackend<S> = crate::cpal::Device<S>;
    pub type BackendSpecificError = crate::cpal::Error;
}

use crate::{ChannelsCount, SampleRate, SamplesCount};
pub use backend_impl::*;

#[allow(dead_code)]
pub(crate) struct CreateBackendConfig {
    /// Backend-specific configuration
    pub backend_specific: BackendSpecificConfig,
    /// Sample rate of the audio stream
    pub sample_rate: SampleRate,
    /// Number of channels in the audio stream
    pub channels: ChannelsCount,
    /// Maximum number of samples in a single block 
    pub buffer_size: SamplesCount,
}

pub(crate) trait BackendDeviceTrait<S>
where
    S: Sample,
{
    fn new(cfg: CreateBackendConfig) -> Result<Self, BackendSpecificError>
    where
        Self: Sized;

    fn open<F>(&mut self, raw_fn: F) -> Result<(), BackendSpecificError>
    where
        F: FnMut(&mut MappedInterleavedBuffer<f32>) + Send + 'static;

    fn close(&mut self) -> Result<(), BackendSpecificError>;
}
