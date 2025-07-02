use crate::{BLOCK_SIZE, CHANNELS_COUNT, DEVICE_BUFFER_SIZE};
use std::ops;

#[derive(Debug)]
pub(crate) enum SampleCode {
    U8,
    I8,
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
    fn to_f32(self) -> f32;

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
                <$type>::from_f32_inner(value)
            }

            fn to_f32(self) -> f32 {
                <$type>::to_f32_inner(self)
            }
        }
    };
}

trait F32Converter {
    // Assume that the value is in the range of -1.0 to 1.0
    fn from_f32_inner(value: f32) -> Self;

    fn to_f32_inner(value: Self) -> f32;
}

impl F32Converter for i8 {
    fn from_f32_inner(value: f32) -> Self {
        (value.clamp(-1.0, 1.0) * (i8::MAX / 2) as f32) as i8
    }

    fn to_f32_inner(value: Self) -> f32 {
        (value as f32) / (i8::MAX / 2) as f32
    }
}

impl F32Converter for u8 {
    fn from_f32_inner(value: f32) -> Self {
        ((value.clamp(-1.0, 1.0) + 1.0) * (u8::MAX as f32 / 2.0)) as u8
    }

    fn to_f32_inner(value: Self) -> f32 {
        ((value as f32) / (u8::MAX as f32 / 2.0)) - 1.0
    }
}

impl F32Converter for i16 {
    fn from_f32_inner(value: f32) -> Self {
        (value.clamp(-1.0, 1.0) * (i16::MAX / 2) as f32) as i16
    }

    fn to_f32_inner(value: Self) -> f32 {
        (value as f32) / (i16::MAX / 2) as f32
    }
}

impl F32Converter for i32 {
    fn from_f32_inner(value: f32) -> Self {
        (value.clamp(-1.0, 1.0) * (i32::MAX / 2) as f32) as i32
    }

    fn to_f32_inner(value: Self) -> f32 {
        (value as f32) / (i32::MAX / 2) as f32
    }
}
impl F32Converter for u16 {
    fn from_f32_inner(value: f32) -> Self {
        ((value.clamp(-1.0, 1.0) + 1.0) * (u16::MAX as f32 / 2.0)) as u16
    }

    fn to_f32_inner(value: Self) -> f32 {
        ((value as f32) / (u16::MAX as f32 / 2.0)) - 1.0
    }
}

impl F32Converter for u32 {
    fn from_f32_inner(value: f32) -> Self {
        ((value.clamp(-1.0, 1.0) + 1.0) * (u32::MAX as f32 / 2.0)) as u32
    }

    fn to_f32_inner(value: Self) -> f32 {
        ((value as f32) / (u32::MAX as f32 / 2.0)) - 1.0
    }
}

impl F32Converter for f32 {
    fn from_f32_inner(value: f32) -> Self {
        value.clamp(-1.0, 1.0)
    }

    fn to_f32_inner(value: Self) -> f32 {
        value.clamp(-1.0, 1.0)
    }
}

impl F32Converter for f64 {
    fn from_f32_inner(value: f32) -> Self {
        value.clamp(-1.0, 1.0) as f64
    }

    fn to_f32_inner(value: Self) -> f32 {
        value.clamp(-1.0, 1.0) as f32
    }
}

impl_sample!(i8, SampleCode::I8);
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

/// This struct represents a buffer of interleaved samples.
/// It is used to store audio samples in a format where
/// each sample contains data for all channels interleaved together.
/// For example: r.0, l.0, r.1, l.1, r.2, l.2, ...
/// Used for endpoint audio processing - passing data to the audio device.
/// The Amount of samples in the buffer is equal to `DEVICE_BUFFER_SIZE`.
#[repr(C)]
#[derive(Debug, Default)]
pub(crate) struct InterleavedSampleBuffer<'a, S>
where
    S: Sample,
{
    pub(crate) samples: &'a mut [InterleavedSample<S>],
    pub(crate) len: usize,
}

impl<'a, S> InterleavedSampleBuffer<'a, S>
where
    S: Sample,
{
    pub fn new(raw: &'a mut [S]) -> Option<Self> {
        // Check that the length of the raw slice is a multiple of CHANNELS_COUNT
        if raw.len() % CHANNELS_COUNT as usize != 0 {
            return None; // Invalid length for interleaved samples
        }

        let ptr = raw.as_mut_ptr() as *mut InterleavedSample<S>;
        let len = raw.len() / CHANNELS_COUNT as usize;

        let casted = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
        Some(Self {
            samples: casted,
            len,
        })
    }
}
