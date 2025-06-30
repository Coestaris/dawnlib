use crate::{BLOCK_SIZE, CHANNELS_COUNT, DEVICE_BUFFER_SIZE};
use std::ops;

#[derive(Debug)]
pub(crate) enum SampleCode {
    I16,
    I32,
    U16,
    U32,
    F32,
    F64,
}

#[allow(dead_code)]
pub(crate) trait Sample: Copy + Clone + PartialOrd + PartialEq + Default {
    fn from_f32(value: f32) -> Self;
    fn zero_value() -> Self;
    fn code() -> SampleCode;
}

macro_rules! impl_sample {
    ($type:ty, $code:expr) => {
        impl Sample for $type {
            fn zero_value() -> Self {
                0 as $type
            }

            fn code() -> SampleCode {
                $code
            }

            fn from_f32(value: f32) -> Self {
                <$type>::clamp_f32(value)
            }
        }
    };
}

trait ClampF32Sample {
    // Assume that the value is in the range of -1.0 to 1.0
    fn clamp_f32(value: f32) -> Self;
}

impl ClampF32Sample for i16 {
    fn clamp_f32(value: f32) -> Self {
        (value.clamp(-1.0, 1.0) * (i16::MAX / 2) as f32) as i16
    }
}

impl ClampF32Sample for i32 {
    fn clamp_f32(value: f32) -> Self {
        (value.clamp(-1.0, 1.0) * (i32::MAX / 2) as f32) as i32
    }
}
impl ClampF32Sample for u16 {
    fn clamp_f32(value: f32) -> Self {
        ((value.clamp(-1.0, 1.0) + 1.0) * (u16::MAX as f32 / 2.0)) as u16
    }
}

impl ClampF32Sample for u32 {
    fn clamp_f32(value: f32) -> Self {
        ((value.clamp(-1.0, 1.0) + 1.0) * (u32::MAX as f32 / 2.0)) as u32
    }
}

impl ClampF32Sample for f32 {
    fn clamp_f32(value: f32) -> Self {
        value.clamp(-1.0, 1.0)
    }
}

impl ClampF32Sample for f64 {
    fn clamp_f32(value: f32) -> Self {
        value.clamp(-1.0, 1.0) as f64
    }
}

impl_sample!(i16, SampleCode::I16);
impl_sample!(i32, SampleCode::I32);
impl_sample!(u16, SampleCode::U16);
impl_sample!(u32, SampleCode::U32);
impl_sample!(f32, SampleCode::F32);
impl_sample!(f64, SampleCode::F64);

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub(crate) struct InterleavedSample<S>
where
    S: Sample,
{
    pub channels: [S; CHANNELS_COUNT as usize],
}

/// This struct represents a buffer of planar samples.
/// It is used to store audio samples in a format where
/// each channel's samples are stored separately.
/// For example: r.0, r.1, r.2, ..., l.0, l.1, l.2, ...
/// Used in audio processing chains - for example, in generators.
/// The Amount of samples in the buffer is equal to `BLOCK_SIZE`.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct PlanarBlock<S>
where
    S: Sample,
{
    pub(crate) samples: [[S; BLOCK_SIZE]; CHANNELS_COUNT as usize],
}

impl<S> Default for PlanarBlock<S>
where
    S: Sample,
{
    fn default() -> Self {
        Self {
            samples: [[S::zero_value(); BLOCK_SIZE]; CHANNELS_COUNT as usize],
        }
    }
}

impl<S> PlanarBlock<S>
where
    S: Sample,
{
    pub(crate) fn silence(&mut self) {
        for channel in self.samples.iter_mut() {
            for sample in channel.iter_mut() {
                *sample = S::zero_value();
            }
        }
    }
}
