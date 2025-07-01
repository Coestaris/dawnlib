use std::fmt::{Display, Formatter};
use yage2_core::threads::ThreadError;
use crate::backend::BackendSpecificError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceCreationError {
    InvalidSampleRate(u32),
    InvalidChannels(u8),
    InvalidBufferSize(usize),
    BackendSpecific(BackendSpecificError),
}

impl Display for DeviceCreationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceCreationError::InvalidSampleRate(rate) => {
                write!(f, "Invalid sample rate: {}", rate)
            }
            DeviceCreationError::InvalidChannels(channels) => {
                write!(f, "Invalid number of channels: {}", channels)
            }
            DeviceCreationError::InvalidBufferSize(size) => {
                write!(f, "Invalid buffer size: {}", size)
            }
            DeviceCreationError::BackendSpecific(err) => {
                write!(f, "Device allocation failed: {}", err)
            }
        }
    }
}

impl std::error::Error for DeviceCreationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceOpenError {
    GeneratorThreadSpawnError(ThreadError),
    EventThreadSpawnError(ThreadError),
    StatisticsThreadSpawnError(ThreadError),
    BackendSpecific(BackendSpecificError),
}

impl Display for DeviceOpenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceOpenError::GeneratorThreadSpawnError(err) => {
                write!(f, "Failed to spawn generator thread: {}", err)
            }
            DeviceOpenError::EventThreadSpawnError(err) => {
                write!(f, "Failed to spawn event thread: {}", err)
            }
            DeviceOpenError::StatisticsThreadSpawnError(err) => {
                write!(f, "Failed to spawn statistics thread: {}", err)
            }
            DeviceOpenError::BackendSpecific(err) => write!(f, "Failed to open device: {}", err),
        }
    }
}

impl std::error::Error for DeviceOpenError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceCloseError {
    BackendSpecific(BackendSpecificError),
}

impl Display for DeviceCloseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceCloseError::BackendSpecific(err) => write!(f, "Failed to close device: {}", err),
        }
    }
}

impl std::error::Error for DeviceCloseError {}
