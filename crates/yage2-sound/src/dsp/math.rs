use crate::dsp::math::basic::{pan_gain_fallback, phase_clamp};
use crate::sample::PlanarBlock;
use crate::{SamplesCount, BLOCK_SIZE, CHANNELS_COUNT};
use std::arch::is_aarch64_feature_detected;

mod mix {
    use crate::sample::PlanarBlock;
    use crate::{BLOCK_SIZE, CHANNELS_COUNT};

    #[inline(never)]
    #[target_feature(enable = "neon")]
    pub unsafe fn neon(input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>) {
        // NEON intrinsics for ARM architecture
        use core::arch::aarch64::*;

        for channel in 0..CHANNELS_COUNT {
            let mut i = 0;
            while i < BLOCK_SIZE {
                // Load 8 samples from both blocks (16 bytes)
                let a = vld1q_f32(input.samples[channel].as_ptr().add(i));
                let b = vld1q_f32(input.samples[channel].as_ptr().add(i));

                // Add the samples
                let result = vaddq_f32(a, b);

                // Store the result back
                vst1q_f32(output.samples[channel].as_mut_ptr().add(i), result);

                i += 4; // Process 8 samples at a time (4 for each)
            }
        }
    }

    #[inline(always)]
    pub fn fallback(input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>) {
        for channel in 0..CHANNELS_COUNT as usize {
            for i in 0..BLOCK_SIZE {
                output.samples[channel][i] = input.samples[channel][i];
            }
        }
    }
}

mod basic {
    use crate::sample::{PlanarBlock, LEFT_CHANNEL, RIGHT_CHANNEL};
    use crate::BLOCK_SIZE;

    #[inline(never)]
    #[target_feature(enable = "neon")]
    pub unsafe fn pan_gain_neon(block: &mut PlanarBlock<f32>, pan: f32, gain: f32) {
        use core::arch::aarch64::*;

        let left_gain = gain * (1.0 - pan).sqrt();
        let right_gain = gain * (1.0 + pan).sqrt();

        let mut i = 0;
        while i < BLOCK_SIZE {
            // Load 4 samples from both channels
            let left_samples = vld1q_f32(block.samples[LEFT_CHANNEL].as_ptr().add(i));
            let right_samples = vld1q_f32(block.samples[RIGHT_CHANNEL].as_ptr().add(i));

            // Scale the samples by the respective gains
            let scaled_left = vmulq_n_f32(left_samples, left_gain);
            let scaled_right = vmulq_n_f32(right_samples, right_gain);

            // Store the results back to the samples
            vst1q_f32(block.samples[LEFT_CHANNEL].as_mut_ptr().add(i), scaled_left);
            vst1q_f32(
                block.samples[RIGHT_CHANNEL].as_mut_ptr().add(i),
                scaled_right,
            );

            i += 4; // Process 4 samples at a time
        }
    }

    #[inline(always)]
    pub fn phase_clamp(block: &mut PlanarBlock<f32>, invert_phase: bool) {
        let phase = if invert_phase { -1.0 } else { 1.0 };

        for i in 0..BLOCK_SIZE {
            block.samples[LEFT_CHANNEL][i] =
                (block.samples[LEFT_CHANNEL][i] * phase).clamp(-1.0, 1.0);
            block.samples[RIGHT_CHANNEL][i] =
                (block.samples[RIGHT_CHANNEL][i] * phase).clamp(-1.0, 1.0);
        }
    }

    #[inline(always)]
    pub fn pan_gain_fallback(block: &mut PlanarBlock<f32>, pan: f32, gain: f32) {
        let left_gain = gain * (1.0 - pan).sqrt();
        let right_gain = gain * (1.0 + pan).sqrt();

        // TODO: Maybe not mixing channels here, may get a performance boost
        //       because of cache hits
        for i in 0..BLOCK_SIZE {
            block.samples[LEFT_CHANNEL][i] = block.samples[LEFT_CHANNEL][i] * left_gain;
            block.samples[RIGHT_CHANNEL][i] = block.samples[RIGHT_CHANNEL][i] * right_gain;
        }
    }
}

// TODO: Implement SIMD support for the Bus
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
    pub(crate) fn copy_from(&mut self, input: &PlanarBlock<f32>) {
        for channel in 0..CHANNELS_COUNT {
            let src_ptr = input.samples[channel].as_ptr();
            let dst_ptr = self.samples[channel].as_mut_ptr();

            unsafe {
                std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, BLOCK_SIZE);
            }
        }
    }

    #[inline(always)]
    pub(crate) fn mix(&mut self, input: &PlanarBlock<f32>) {
        if is_aarch64_feature_detected!("neon") {
            unsafe {
                mix::neon(input, self);
            }
        } else {
            mix::fallback(input, self);
        }
    }

    #[inline(always)]
    pub(crate) fn pan_gain_phase_clamp(&mut self, pan: f32, gain: f32, invert_phase: bool) {
        if is_aarch64_feature_detected!("neon") {
            unsafe {
                basic::pan_gain_neon(self, pan, gain);
            }
            basic::phase_clamp(self, invert_phase);
        } else {
            // TODO: Implement benches to compare performance
            pan_gain_fallback(self, pan, gain);
            phase_clamp(self, invert_phase);
        }
    }
}
