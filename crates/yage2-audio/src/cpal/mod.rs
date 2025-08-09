use crate::backend::{InternalBackendConfig, PlayerBackendTrait};
use crate::sample::{MappedInterleavedBuffer, Sample, SampleCode};
use crate::{ChannelsCount, SampleRate, SamplesCount};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SizedSample;
use log::{debug, info, warn};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    NotSupportedStreamParameters(SampleRate, ChannelsCount, SamplesCount, cpal::SampleFormat),
    FetchConfigFailed(cpal::SupportedStreamConfigsError),
    BuildStreamError(cpal::BuildStreamError),
    StartStreamError(cpal::PlayStreamError),
    AlreadyOpened,
    AlreadyClosed,
    PausedStreamError(cpal::PauseStreamError),
    DefaultHostNotFound,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotSupportedStreamParameters(
                sample_rate,
                channels,
                buffer_size,
                sample_format,
            ) => {
                write!(
                    f,
                    "Stream parameters not supported: sample_rate: {}, channels: {}, buffer_size: {}, sample_format: {:?}",
                    sample_rate, channels, buffer_size, sample_format
                )
            }
            Error::StartStreamError(err) => write!(f, "Failed to start stream: {}", err),
            Error::PausedStreamError(err) => write!(f, "Failed to pause stream: {}", err),
            Error::AlreadyOpened => write!(f, "Stream is already opened"),
            Error::AlreadyClosed => write!(f, "Stream is already closed"),
            Error::BuildStreamError(err) => write!(f, "Failed to build stream: {}", err),
            Error::DefaultHostNotFound => write!(f, "Default host not found"),
            Error::FetchConfigFailed(err) => write!(f, "Failed to fetch config: {}", err),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub struct PlayerConfig {}

pub(crate) struct Player<S> {
    device: cpal::Device,
    stream_config: cpal::StreamConfig,
    stream: Option<cpal::Stream>,
    keep_s: PhantomData<S>,
}

#[cfg(target_os = "macos")]
// Internal CPAL implementation for MacOS has weird issues with Send
// and Sync traits. I'll deal with it later.
unsafe impl<S> Send for Player<S> where S: Sample + Send {}

fn sample_code_to_cpal_format<S>() -> cpal::SampleFormat
where
    S: Sample,
{
    match S::code() {
        crate::sample::SampleCode::I16 => cpal::SampleFormat::I16,
        crate::sample::SampleCode::I32 => cpal::SampleFormat::I32,
        crate::sample::SampleCode::F32 => cpal::SampleFormat::F32,
        crate::sample::SampleCode::F64 => cpal::SampleFormat::F64,
        _ => panic!("Unsupported sample type for cpal: {:?}", S::code()),
    }
}

impl<S> PlayerBackendTrait<S> for Player<S>
where
    S: Sample + SizedSample + Send,
{
    fn new(cfg: InternalBackendConfig) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(Error::DefaultHostNotFound)?;

        let supported_configs = device
            .supported_output_configs()
            .map_err(Error::FetchConfigFailed)?;

        let mut selected_config: Option<cpal::StreamConfig> = None;
        let required_sample_format = sample_code_to_cpal_format::<S>();

        #[cfg(target_os = "macos")]
        // In some reason I can't explain, on macOS the buffer size is some
        // random value bigger than the requested one, so we limit it to
        // 80% of the requested size. In the other case, the upper level code
        // will panic.
        let max_buf_size = (cfg.buffer_size as f32 * 0.8) as usize;
        #[cfg(not(target_os = "macos"))]
        let max_buf_size = cfg.buffer_size as usize;

        for config in supported_configs {
            // Log the supported config details
            debug!(
                "Supported config: sample_format: {:?}, sample_rate: {}-{}, channels: {}, buffer_size: {:?}",
                config.sample_format(),
                config.min_sample_rate().0,
                config.max_sample_rate().0,
                config.channels(),
                config.buffer_size());

            let sample_format_ok = config.sample_format() == required_sample_format;
            let sample_rate_ok = config.min_sample_rate()
                <= cpal::SampleRate(cfg.sample_rate as u32)
                && cpal::SampleRate(cfg.sample_rate as u32) <= config.max_sample_rate();
            let channels_ok = config.channels() == cfg.channels as u16;
            let buffer_size_ok = match config.buffer_size() {
                cpal::SupportedBufferSize::Range { min, max } => {
                    max_buf_size >= *min as usize && max_buf_size <= *max as usize
                }
                cpal::SupportedBufferSize::Unknown => continue,
            };

            debug!(
                "Checking config: sample_format_ok: {}, sample_rate_ok: {}, channels_ok: {}, buffer_size_ok: {}",
                sample_format_ok, sample_rate_ok, channels_ok, buffer_size_ok
            );

            if sample_format_ok && sample_rate_ok && channels_ok && buffer_size_ok {
                selected_config = Some(cpal::StreamConfig {
                    channels: config.channels(),
                    sample_rate: cpal::SampleRate(cfg.sample_rate as u32),
                    buffer_size: cpal::BufferSize::Fixed(max_buf_size as u32),
                });
                break;
            }
        }

        Ok(Player::<S> {
            device,
            stream_config: selected_config.ok_or(Error::NotSupportedStreamParameters(
                cfg.sample_rate,
                cfg.channels,
                cfg.buffer_size,
                required_sample_format,
            ))?,
            stream: None,

            keep_s: Default::default(),
        })
    }

    fn open<F>(&mut self, mut raw_fn: F) -> Result<(), Error>
    where
        F: FnMut(&mut MappedInterleavedBuffer<f32>) + Send + 'static,
    {
        if self.stream.is_some() {
            return Err(Error::AlreadyOpened);
        }

        info!("Opening stream");
        let err_fn = |err| eprintln!("Error building output sound stream: {}", err);
        let interleaved_samples_count = match self.stream_config.buffer_size {
            cpal::BufferSize::Fixed(size) => size as usize,
            cpal::BufferSize::Default => {
                panic!("Default buffer size is not supported in this context")
            }
        };
        let samples_count = interleaved_samples_count * self.stream_config.channels as usize;

        let f = match S::code() {
            SampleCode::I8 => {
                todo!()
            }
            SampleCode::I16 => {
                todo!()
            }
            SampleCode::I24 => {
                todo!()
            }
            SampleCode::I32 => {
                todo!()
            }
            SampleCode::U16 => {
                todo!()
            }
            SampleCode::U32 => {
                todo!()
            }
            SampleCode::F32 => {
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| match MappedInterleavedBuffer::<
                    f32,
                >::new(
                    data
                ) {
                    Some(mut mapped_buffer) => {
                        raw_fn(&mut mapped_buffer);
                    }
                    None => {
                        warn!(
                                "Failed to create interleaved sample buffer: expected {} samples, got {}",
                                samples_count,
                                data.len()
                            );
                    }
                }
            }
            SampleCode::F64 => {
                todo!()
            }
        };

        let stream = self
            .device
            .build_output_stream(&self.stream_config, f, err_fn, None)
            .map_err(Error::BuildStreamError)?;

        stream.play().map_err(Error::StartStreamError)?;
        self.stream = Some(stream);

        Ok(())
    }

    fn close(&mut self) -> Result<(), Error> {
        match self.stream {
            Some(ref stream) => {
                stream.pause().map_err(Error::PausedStreamError)?;
                self.stream = None;
                Ok(())
            }
            None => Err(Error::AlreadyClosed),
        }
    }
}
