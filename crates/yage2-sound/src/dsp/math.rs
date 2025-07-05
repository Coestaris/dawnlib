use log::info;
use crate::sample::PlanarBlock;
use crate::{BLOCK_SIZE, CHANNELS_COUNT};

// TODO: Implement SIMD support for the Bus
impl PlanarBlock<f32> {
    #[inline(always)]
    pub(crate) fn copy_from_planar_vec(
        &mut self,
        input: &[Vec<f32>; CHANNELS_COUNT as usize],
        start: usize,
        size: usize,
    ) {
        for channel in 0..self.samples.len() {
            let src = &input[channel][start..start + size];
            let dst = &mut self.samples[channel];
            dst.copy_from_slice(src);
        }
    }

    #[inline(always)]
    pub(crate) fn copy_from(&mut self, input: &PlanarBlock<f32>) {
        unsafe {
           for channel in 0..self.samples.len() {
                let src_ptr = input.samples[channel].as_ptr();
                let dst_ptr = self.samples[channel].as_mut_ptr();
                let len = input.samples[channel].len();

                // Use `copy_nonoverlapping` for safe copying
                std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, len);
            }
        }
    }

    #[inline(never)]
    #[target_feature(enable = "neon")]
    pub fn mix_simd(&mut self, input: &PlanarBlock<f32>) {
        // NEON intrinsics for ARM architecture
        use core::arch::aarch64::*;

        unsafe {
            for channel in 0..self.samples.len() {
                let mut i = 0;
                while i < BLOCK_SIZE {
                    // Load 8 samples from both blocks (16 bytes)
                    let a = vld1q_f32(self.samples[channel].as_ptr().add(i));
                    let b = vld1q_f32(input.samples[channel].as_ptr().add(i));

                    // Add the samples
                    let result = vaddq_f32(a, b);

                    // Store the result back
                    vst1q_f32(self.samples[channel].as_mut_ptr().add(i), result);

                    i += 4; // Process 8 samples at a time (4 for each)
                }
            }
        }
    }

    pub(crate) fn mix(&mut self, input: &PlanarBlock<f32>) {
        for channel in 0..self.samples.len() {
            for i in 0..self.samples[channel].len() {
                self.samples[channel][i] += input.samples[channel][i];
            }
        }
    }

    #[inline(never)]
    #[target_feature(enable = "neon")]
    pub(crate) fn pan_gain_simd(&mut self, pan: f32, gain: f32) {
        // NEON intrinsics for ARM architecture
        use core::arch::aarch64::*;

        unsafe {
            let left_gain = gain * (1.0 - pan).sqrt();
            let right_gain = gain * (1.0 + pan).sqrt();

            let mut i = 0;
            while i < BLOCK_SIZE {
                // Load 4 samples from both channels
                let left_samples = vld1q_f32(self.samples[0].as_ptr().add(i));
                let right_samples = vld1q_f32(self.samples[1].as_ptr().add(i));

                // Scale the samples by the respective gains
                let scaled_left = vmulq_n_f32(left_samples, left_gain);
                let scaled_right = vmulq_n_f32(right_samples, right_gain);

                // Store the results back to the samples
                vst1q_f32(self.samples[0].as_mut_ptr().add(i), scaled_left);
                vst1q_f32(self.samples[1].as_mut_ptr().add(i), scaled_right);

                i += 4; // Process 4 samples at a time
            }
        }
    }

    pub(crate) fn pan_gain_phase_clamp(&mut self, pan: f32, gain: f32, invert_phase: bool) {
        let left_gain = gain * (1.0 - pan).sqrt();
        let right_gain = gain * (1.0 + pan).sqrt();
        let phase = if invert_phase { -1.0 } else { 1.0 };

        for i in 0..BLOCK_SIZE {
            self.samples[0][i] = (self.samples[0][i] * left_gain * phase).clamp(-1.0, 1.0);
            self.samples[1][i] = (self.samples[1][i] * right_gain * phase).clamp(-1.0, 1.0);
        }
    }
}
