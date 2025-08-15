use crate::sample::{MappedInterleavedBuffer, Sample};

pub mod backend_impl {
    pub type PlayerBackendConfig = crate::cpal::PlayerConfig;
    pub(crate) type PlayerBackend<S> = crate::cpal::Player<S>;
    pub type PlayerBackendError = crate::cpal::Error;
}

use crate::{ChannelsCount, SampleRate, SamplesCount};
pub use backend_impl::*;

#[allow(dead_code)]
pub(crate) struct InternalBackendConfig {
    /// Backend-specific configuration
    pub backend_specific: PlayerBackendConfig,
    /// Sample rate of the audio stream
    pub sample_rate: SampleRate,
    /// Number of channels in the audio stream
    pub channels: ChannelsCount,
    /// Maximum number of samples in a single block
    pub buffer_size: SamplesCount,
}

pub(crate) trait PlayerBackendTrait<S>
where
    S: Sample,
{
    fn new(cfg: InternalBackendConfig) -> Result<Self, PlayerBackendError>
    where
        Self: Sized;

    fn open<F>(&mut self, raw_fn: F) -> Result<(), PlayerBackendError>
    where
        F: FnMut(&mut MappedInterleavedBuffer<f32>) + Send + 'static;

    fn close(&mut self) -> Result<(), PlayerBackendError>;
}
