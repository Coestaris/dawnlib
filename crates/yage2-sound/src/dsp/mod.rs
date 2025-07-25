use crate::sample::{InterleavedBlock, PlanarBlock, LEFT_CHANNEL, RIGHT_CHANNEL};
use crate::{SamplesCount, BLOCK_SIZE, CHANNELS_COUNT};

mod add;
mod addm;
mod copy_into_interleaved;
mod fir;
mod soft_clip;
mod soft_limit;
#[cfg(test)]
mod tests;

#[cfg(target_arch = "x86_64")]
mod features {
    use log::debug;

    pub static X86_HAS_SSE42: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    pub static X86_HAS_AVX: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    pub static X86_HAS_AVX2: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    pub static X86_HAS_AVX512: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

    pub fn detect_features() {
        use std::arch::is_x86_feature_detected;
        X86_HAS_SSE42.set(is_x86_feature_detected!("sse4.2"));
        X86_HAS_AVX.set(is_x86_feature_detected!("avx"));
        X86_HAS_AVX2.set(is_x86_feature_detected!("avx2"));
        X86_HAS_AVX512.set(is_x86_feature_detected!("avx512f"));

        debug!("Detecting x86 features (by priority):");
        debug!("AVX512: {}", X86_HAS_AVX512.get().unwrap());
        debug!("AVX2: {}", X86_HAS_AVX2.get().unwrap());
        debug!("AVX: {}", X86_HAS_AVX.get().unwrap());
        debug!("SSE2: {}", X86_HAS_SSE42.get().unwrap());
    }
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
mod features {
    use log::debug;

    pub static ARM_HAS_NEON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    pub static ARM_HAS_SVE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

    pub fn detect_features() {
        use std::arch::is_aarch64_feature_detected;
        let _ = ARM_HAS_NEON.set(is_aarch64_feature_detected!("neon"));
        let _ = ARM_HAS_SVE.set(is_aarch64_feature_detected!("sve"));

        debug!("Detecting ARM features (by priority):");
        debug!("SVE: {}", ARM_HAS_SVE.get().unwrap());
        debug!("NEON: {}", ARM_HAS_NEON.get().unwrap());
    }
}

pub use features::detect_features;
use features::*;

macro_rules! call_accelerated(
    ($arch:expr, $align:expr, $condvar:ident, $func:expr, $($args:expr),*) => {
        #[cfg(target_arch = $arch)]
        if BLOCK_SIZE % $align == 0 && *$condvar.get().unwrap() {
            return unsafe {
                $func($($args),*);
            };
        }

    }
);

impl PlanarBlock<f32> {
    #[inline(always)]
    pub(crate) fn copy_from_planar_vec(
        &mut self,
        input: &[Vec<f32>; CHANNELS_COUNT],
        start: SamplesCount,
        size: SamplesCount,
    ) {
        for channel in 0..CHANNELS_COUNT {
            let src = &input[channel][start..start + size];
            let dst = &mut self.samples[channel];
            dst.copy_from_slice(src);
        }
    }

    #[inline(always)]
    pub(crate) fn silence(&mut self) {
        // Fill each channel with zeros
        for channel in 0..CHANNELS_COUNT {
            let dst_ptr = self.samples[channel].as_mut_ptr();
            unsafe {
                std::ptr::write_bytes(dst_ptr, 0, BLOCK_SIZE);
            }
        }
    }

    #[inline(always)]
    pub(crate) fn copy_from(&mut self, input: &PlanarBlock<f32>) {
        // If the input and self are the same, we can skip copying
        // (check if they are the same reference)
        let addr_self = self as *const _;
        let addr_input = input as *const _;
        if addr_self == addr_input {
            return;
        }

        // Copy samples from input to self for each channel
        for channel in 0..CHANNELS_COUNT {
            let src_ptr = input.samples[channel].as_ptr();
            let dst_ptr = self.samples[channel].as_mut_ptr();

            unsafe {
                std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, BLOCK_SIZE);
            }
        }
    }

    #[inline(always)]
    pub(crate) fn copy_into_interleaved(
        self: &PlanarBlock<f32>,
        output: &mut InterleavedBlock<f32>,
    ) {
        macro_rules! accelerated(
            ($arch:expr, $align:expr, $condvar:ident, $func:expr) => {
                call_accelerated!(
                    $arch,
                    $align,
                    $condvar,
                    $func,
                    self,
                    output
                );
            }
        );

        use copy_into_interleaved::*;
        // accelerated!("aarch64", 4, ARM_HAS_NEON, neon_block_m4);
        // accelerated!("aarch64", 4, ARM_HAS_SVE, sve_block_m4);
        accelerated!("x86_64", 32, X86_HAS_AVX512, avx512_block_m32);
        accelerated!("x86_64", 32, X86_HAS_AVX2, avx2_block_m32);
        accelerated!("x86_64", 32, X86_HAS_AVX, avx_block_m32);
        accelerated!("x86_64", 32, X86_HAS_SSE42, sse42_block_m32);

        // Fallback to the basic implementation if SIMD is not available
        fallback(self, output);
    }

    #[inline(always)]
    pub(crate) fn add(&mut self, input: &PlanarBlock<f32>) {
        macro_rules! accelerated(
            ($arch:expr, $align:expr, $condvar:ident, $func:expr) => {
                call_accelerated!(
                    $arch,
                    $align,
                    $condvar,
                    $func,
                    &input,
                    self
                );
            }
        );

        use add::*;
        accelerated!("aarch64", 4, ARM_HAS_NEON, neon_block_m4);
        accelerated!("aarch64", 4, ARM_HAS_SVE, sve_block_m4);
        accelerated!("x86_64", 32, X86_HAS_AVX512, avx512_block_m32);
        accelerated!("x86_64", 32, X86_HAS_AVX2, avx2_block_m32);
        accelerated!("x86_64", 32, X86_HAS_AVX, avx_block_m32);
        accelerated!("x86_64", 32, X86_HAS_SSE42, sse42_block_m32);

        // Fallback to the basic implementation if SIMD is not available
        fallback(input, self);
    }

    #[inline(always)]
    pub(crate) fn addm(&mut self, input: &PlanarBlock<f32>, k: f32) {
        macro_rules! accelerated(
            ($arch:expr, $align:expr, $condvar:ident, $func:expr) => {
                call_accelerated!(
                    $arch,
                    $align,
                    $condvar,
                    $func,
                    &input,
                    self,
                    k
                );
            }
        );

        use addm::*;
        accelerated!("aarch64", 4, ARM_HAS_NEON, neon_block_m4);
        accelerated!("aarch64", 4, ARM_HAS_SVE, sve_block_m4);
        accelerated!("x86_64", 32, X86_HAS_AVX512, avx512_block_m32);
        accelerated!("x86_64", 32, X86_HAS_AVX2, avx2_block_m32);
        accelerated!("x86_64", 32, X86_HAS_AVX, avx_block_m32);
        accelerated!("x86_64", 32, X86_HAS_SSE42, sse42_block_m32);

        // Fallback to the basic implementation if SIMD is not available
        fallback(input, self, k);
    }

    #[inline(always)]
    pub(crate) fn soft_clip(&mut self) {
        macro_rules! accelerated(
            ($arch:expr, $align:expr, $condvar:ident, $func:expr) => {
                call_accelerated!(
                    $arch,
                    $align,
                    $condvar,
                    $func,
                    self
                );
            }
        );

        // use soft_clip::*;
        // accelerated!("aarch64", 4, ARM_HAS_NEON, neon_block_m4);
        // accelerated!("aarch64", 4, ARM_HAS_SVE, sve_block_m4);
        // accelerated!("x86_64", 32, X86_HAS_AVX512, avx512_block_m32);
        // accelerated!("x86_64", 32, X86_HAS_AVX2, avx2_block_m32);
        // accelerated!("x86_64", 32, X86_HAS_AVX, avx_block_m32);
        // accelerated!("x86_64", 32, X86_HAS_SSE42, sse42_block_m32);
        //
        // // Fallback to the basic implementation if no SIMD is available
        // fallback(self);
    }

    #[inline(always)]
    pub(crate) fn gain_pan(&mut self, gain: f32, pan: f32) {
        // TODO: Implement SIMD acceleration for gain and pan
        // Separating the channels into two loops gives a little bit
        // better performance due to better cache locality
        let left_gain = gain * (1.0 - pan).sqrt();
        for i in 0..BLOCK_SIZE {
            self.samples[LEFT_CHANNEL][i] = (self.samples[LEFT_CHANNEL][i] * left_gain);
        }

        let right_gain = gain * (1.0 + pan).sqrt();
        for i in 0..BLOCK_SIZE {
            self.samples[RIGHT_CHANNEL][i] = (self.samples[RIGHT_CHANNEL][i] * right_gain);
        }
    }
}
