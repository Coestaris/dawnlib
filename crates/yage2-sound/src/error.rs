use std::fmt::{Display, Formatter};
use yage2_core::threads::ThreadError;
use crate::backend::BackendSpecificError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioManagerCreationError {
    InvalidSampleRate(u32),
    InvalidChannels(u8),
    InvalidBufferSize(usize),
    BackendSpecific(BackendSpecificError),
}

impl Display for AudioManagerCreationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioManagerCreationError::InvalidSampleRate(rate) => {
                write!(f, "Invalid sample rate: {}", rate)
            }
            AudioManagerCreationError::InvalidChannels(channels) => {
                write!(f, "Invalid number of channels: {}", channels)
            }
            AudioManagerCreationError::InvalidBufferSize(size) => {
                write!(f, "Invalid buffer size: {}", size)
            }
            AudioManagerCreationError::BackendSpecific(err) => {
                write!(f, "Device allocation failed: {}", err)
            }
        }
    }
}

impl std::error::Error for AudioManagerCreationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioManagerStartError {
    GeneratorThreadSpawnError(ThreadError),
    EventThreadSpawnError(ThreadError),
    StatisticsThreadSpawnError(ThreadError),
    BackendSpecific(BackendSpecificError),
}

impl Display for AudioManagerStartError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioManagerStartError::GeneratorThreadSpawnError(err) => {
                write!(f, "Failed to spawn generator thread: {}", err)
            }
            AudioManagerStartError::EventThreadSpawnError(err) => {
                write!(f, "Failed to spawn event thread: {}", err)
            }
            AudioManagerStartError::StatisticsThreadSpawnError(err) => {
                write!(f, "Failed to spawn statistics thread: {}", err)
            }
            AudioManagerStartError::BackendSpecific(err) => write!(f, "Failed to open device: {}", err),
        }
    }
}

impl std::error::Error for AudioManagerStartError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioManagerStopError {
    BackendSpecific(BackendSpecificError),
}

impl Display for AudioManagerStopError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioManagerStopError::BackendSpecific(err) => write!(f, "Failed to close device: {}", err),
        }
    }
}

impl std::error::Error for AudioManagerStopError {}
