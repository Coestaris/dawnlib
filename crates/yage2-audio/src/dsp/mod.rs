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

#[cfg(test)]
mod feature_flag_impl {
    use std::cell::UnsafeCell;

    #[cfg(test)]
    // Allow overriding feature flags in tests
    pub(crate) struct FeatureFlag {
        value: UnsafeCell<bool>,
    }

    impl FeatureFlag {
        pub const fn new() -> Self {
            FeatureFlag {
                value: UnsafeCell::new(false),
            }
        }

        pub fn get(&self) -> bool {
            unsafe { *self.value.get() }
        }

        pub fn set(&self, value: bool) {
            unsafe {
                *self.value.get() = value;
            }
        }
    }
    unsafe impl Sync for FeatureFlag {}
    unsafe impl Send for FeatureFlag {}
}

#[cfg(not(test))]
mod feature_flag_impl {
    use std::sync::OnceLock;

    #[cfg(not(test))]
    pub(crate) struct FeatureFlag {
        value: OnceLock<bool>,
    }

    #[cfg(not(test))]
    impl FeatureFlag {
        pub const fn new() -> Self {
            FeatureFlag {
                value: std::sync::OnceLock::new(),
            }
        }

        pub fn get(&self) -> bool {
            *self.value.get().unwrap_or(&false)
        }

        pub fn set(&self, value: bool) {
            let _ = self.value.set(value);
        }
    }
}

use feature_flag_impl::FeatureFlag;

#[cfg(target_arch = "x86_64")]
mod features {
    use crate::dsp::FeatureFlag;
    use log::debug;

    pub static X86_HAS_SSE42: FeatureFlag = FeatureFlag::new();
    pub static X86_HAS_AVX: FeatureFlag = FeatureFlag::new();
    pub static X86_HAS_AVX2: FeatureFlag = FeatureFlag::new();
    pub static X86_HAS_AVX512: FeatureFlag = FeatureFlag::new();

    pub fn detect_features() {
        use std::arch::is_x86_feature_detected;
        X86_HAS_SSE42.set(is_x86_feature_detected!("sse4.2"));
        X86_HAS_AVX.set(is_x86_feature_detected!("avx"));
        X86_HAS_AVX2.set(is_x86_feature_detected!("avx2"));
        X86_HAS_AVX512.set(is_x86_feature_detected!("avx512f"));

        debug!("Detecting x86 features (by priority):");
        debug!("AVX512: {}", X86_HAS_AVX512.get());
        debug!("AVX2: {}", X86_HAS_AVX2.get());
        debug!("AVX: {}", X86_HAS_AVX.get());
        debug!("SSE2: {}", X86_HAS_SSE42.get());
    }

    #[cfg(test)]
    pub fn disable_all_features() {
        X86_HAS_SSE42.set(false);
        X86_HAS_AVX.set(false);
        X86_HAS_AVX2.set(false);
        X86_HAS_AVX512.set(false);

        debug!("All x86 features disabled for testing.");
    }
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
mod features {
    use crate::dsp::FeatureFlag;
    use log::debug;

    pub static ARM_HAS_NEON: FeatureFlag = FeatureFlag::new();
    pub static ARM_HAS_SVE: FeatureFlag = FeatureFlag::new();

    pub fn detect_features() {
        use std::arch::is_aarch64_feature_detected;
        let _ = ARM_HAS_NEON.set(is_aarch64_feature_detected!("neon"));
        let _ = ARM_HAS_SVE.set(is_aarch64_feature_detected!("sve"));

        debug!("Detecting ARM features (by priority):");
        debug!("SVE: {}", ARM_HAS_SVE.get());
        debug!("NEON: {}", ARM_HAS_NEON.get());
    }

    #[cfg(test)]
    pub fn disable_all_features() {
        ARM_HAS_NEON.set(false);
        ARM_HAS_SVE.set(false);

        debug!("All ARM features disabled for testing.");
    }
}

pub use features::detect_features;
#[cfg(test)]
pub use features::disable_all_features;
use features::*;

macro_rules! call_accelerated(
        ($arch:expr, $align:expr, $condvar:ident, $func:expr, $($args:expr),*) => {
            #[cfg(target_arch = $arch)]
            if BLOCK_SIZE % $align == 0 && $condvar.get() {
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
        // accelerated!("x86_64", 32, X86_HAS_AVX512, avx512_block_m32); TODO: Sometimes causes a crash
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
