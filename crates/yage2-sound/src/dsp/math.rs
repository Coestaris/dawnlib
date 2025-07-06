use crate::sample::PlanarBlock;
use crate::{SamplesCount, BLOCK_SIZE, CHANNELS_COUNT};
use log::info;
use std::arch::is_aarch64_feature_detected;
mod mix {
    use crate::sample::PlanarBlock;
    use crate::{BLOCK_SIZE, CHANNELS_COUNT};

    #[inline(never)]
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    // Note, that block adrress MUST be aligned to 32 bytes
    //       and BLOCK_SIZE must be a multiple of 8
    pub unsafe fn avx2_block_m32(input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>) {
        // AVX2 intrinsics for x86_64 architecture
        use core::arch::x86_64::*;

        for channel in 0..CHANNELS_COUNT {
            let input_ptr = input.samples.get_unchecked(channel).as_ptr();
            let output_ptr = output.samples.get_unchecked_mut(channel).as_mut_ptr();

            debug_assert_eq!(
                (input_ptr as usize) % 32,
                0,
                "input_ptr not 32-byte aligned"
            );

            // 128 floats / 8 = 16 vectors
            // Unroll x4: 16 / 4 = 4 iterations
            let mut i = 0;
            while i < BLOCK_SIZE {
                let in0 = _mm256_load_ps(input_ptr.add(i));
                let in1 = _mm256_load_ps(input_ptr.add(i + 8));
                let in2 = _mm256_load_ps(input_ptr.add(i + 16));
                let in3 = _mm256_load_ps(input_ptr.add(i + 24));

                let out0 = _mm256_load_ps(output_ptr.add(i));
                let out1 = _mm256_load_ps(output_ptr.add(i + 8));
                let out2 = _mm256_load_ps(output_ptr.add(i + 16));
                let out3 = _mm256_load_ps(output_ptr.add(i + 24));

                _mm256_store_ps(output_ptr.add(i), _mm256_add_ps(in0, out0));
                _mm256_store_ps(output_ptr.add(i + 8), _mm256_add_ps(in1, out1));
                _mm256_store_ps(output_ptr.add(i + 16), _mm256_add_ps(in2, out2));
                _mm256_store_ps(output_ptr.add(i + 24), _mm256_add_ps(in3, out3));

                i += 32;
            }
        }
    }

    #[inline(never)]
    #[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
    #[target_feature(enable = "neon")]
    pub unsafe fn neon(input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>) {
        // NEON intrinsics for ARM architecture
        use core::arch::aarch64::*;

        const _: () = assert!(
            BLOCK_SIZE % 4 == 0,
            "BLOCK_SIZE must be a multiple of 4 for NEON"
        );

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
                output.samples[channel][i] += input.samples[channel][i];
            }
        }
    }
}

mod pan_gain_phase_clamp {
    use crate::sample::{PlanarBlock, LEFT_CHANNEL, RIGHT_CHANNEL};
    use crate::{BLOCK_SIZE, CHANNELS_COUNT};

    #[inline(never)]
    #[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
    #[target_feature(enable = "neon")]
    pub unsafe fn pan_gain_neon(block: &mut PlanarBlock<f32>, pan: f32, gain: f32) {
        use core::arch::aarch64::*;

        let left_gain = gain * (1.0 - pan).sqrt();
        let right_gain = gain * (1.0 + pan).sqrt();

        const _: () = assert!(
            BLOCK_SIZE % 4 == 0,
            "BLOCK_SIZE must be a multiple of 4 for NEON"
        );

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

    #[inline(never)]
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    pub unsafe fn avx2_block_m32(
        block: &mut PlanarBlock<f32>,
        pan: f32,
        gain: f32,
        invert_phase: bool,
    ) {
        use core::arch::x86_64::*;

        let one = _mm256_set1_ps(1.0);
        let neg_one = _mm256_set1_ps(-1.0);

        macro_rules! process_channel {
            ($gain:expr, $channel_index:expr) => {
                let mul = _mm256_set1_ps($gain);
                let mut i = 0;
                while i < BLOCK_SIZE {
                    let in0 = _mm256_loadu_ps(block.samples[$channel_index].as_ptr().add(i));
                    let in1 = _mm256_loadu_ps(block.samples[$channel_index].as_ptr().add(i + 8));
                    let in2 = _mm256_loadu_ps(block.samples[$channel_index].as_ptr().add(i + 16));
                    let in3 = _mm256_loadu_ps(block.samples[$channel_index].as_ptr().add(i + 24));

                    let out0 = _mm256_mul_ps(in0, mul);
                    let out0 = _mm256_min_ps(_mm256_max_ps(out0, neg_one), one); // Clamp to [-1.0, 1.0]

                    let out1 = _mm256_mul_ps(in1, mul);
                    let out1 = _mm256_min_ps(_mm256_max_ps(out1, neg_one), one);

                    let out2 = _mm256_mul_ps(in2, mul);
                    let out2 = _mm256_min_ps(_mm256_max_ps(out2, neg_one), one);

                    let out3 = _mm256_mul_ps(in3, mul);
                    let out3 = _mm256_min_ps(_mm256_max_ps(out3, neg_one), one);

                    _mm256_storeu_ps(block.samples[$channel_index].as_mut_ptr().add(i), out0);
                    _mm256_storeu_ps(block.samples[$channel_index].as_mut_ptr().add(i + 8), out1);
                    _mm256_storeu_ps(block.samples[$channel_index].as_mut_ptr().add(i + 16), out2);
                    _mm256_storeu_ps(block.samples[$channel_index].as_mut_ptr().add(i + 24), out3);

                    i += 32; // Process 32 samples at a time
                }
            }
        }

        // Process left channel
        let left_gain = gain * (1.0 - pan).sqrt() * (if invert_phase { -1.0 } else { 1.0 });
        process_channel!(left_gain, LEFT_CHANNEL);

        // Process right channel
        let right_gain = gain * (1.0 + pan).sqrt() * (if invert_phase { -1.0 } else { 1.0 });
        process_channel!(right_gain, RIGHT_CHANNEL);
    }

    #[inline(always)]
    pub fn fallback(block: &mut PlanarBlock<f32>, pan: f32, gain: f32, invert_phase: bool) {
        // Separating the channels into two loops gives a little bit
        // better performance due to better cache locality
        let left_gain = gain * (1.0 - pan).sqrt() * (if invert_phase { -1.0 } else { 1.0 });
        for i in 0..BLOCK_SIZE {
            block.samples[LEFT_CHANNEL][i] =
                (block.samples[LEFT_CHANNEL][i] * left_gain).clamp(-1.0, 1.0);
        }

        let right_gain = gain * (1.0 + pan).sqrt() * (if invert_phase { -1.0 } else { 1.0 });
        for i in 0..BLOCK_SIZE {
            block.samples[RIGHT_CHANNEL][i] =
                (block.samples[RIGHT_CHANNEL][i] * right_gain).clamp(-1.0, 1.0);
        }
    }
}

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
        #[cfg(target_arch = "aarch64")]
        {
            static HAS_NEON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            let has_neon =
                *HAS_NEON.get_or_init(|| std::arch::is_aarch64_feature_detected!("neon"));
            if has_neon {
                return unsafe {
                    mix::neon(input, self);
                };
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            if BLOCK_SIZE % 32 == 0 {
                static HAS_AVX2: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
                let has_avx2 =
                    *HAS_AVX2.get_or_init(|| std::arch::is_x86_feature_detected!("avx2"));
                if has_avx2 {
                    return unsafe {
                        mix::avx2_block_m32(input, self);
                    };
                }
            }
        }

        // Fallback to the basic implementation if SIMD is not available
        mix::fallback(input, self);
    }

    #[inline(always)]
    pub(crate) fn pan_gain_phase_clamp(&mut self, pan: f32, gain: f32, invert_phase: bool) {
        #[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
        {
            static HAS_NEON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            let has_neon = *HAS_NEON.get_or_init(|| is_aarch64_feature_detected!("neon"));

            if has_neon {
                return unsafe {
                    pan_gain_phase_clamp::neon(self, pan, gain);
                };
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            if BLOCK_SIZE % 32 == 0 {
                static HAS_AVX2: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
                let has_avx2 =
                    *HAS_AVX2.get_or_init(|| std::arch::is_x86_feature_detected!("avx2"));
                if has_avx2 {
                    return unsafe {
                        pan_gain_phase_clamp::avx2_block_m32(self, pan, gain, invert_phase);
                    };
                }
            }
        }

        pan_gain_phase_clamp::fallback(self, pan, gain, invert_phase);
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use std::panic;
    use test::Bencher;

    #[test]
    fn copy_from_planar_vec_full_test() {
        use crate::sample::PlanarBlock;
        use crate::{BLOCK_SIZE, CHANNELS_COUNT};

        let mut block = PlanarBlock::<f32>::default();
        let input: [Vec<f32>; CHANNELS_COUNT] = [
            (0..BLOCK_SIZE).map(|i| i as f32).collect(),
            (0..BLOCK_SIZE).map(|i| i as f32 + 1.0).collect(),
        ];

        block.copy_from_planar_vec(&input, 0, BLOCK_SIZE);

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                assert_eq!(block.samples[channel][i], input[channel][i]);
            }
        }
    }

    #[test]
    fn copy_from_planar_vec_fail_if_not_end_of_block() {
        use crate::sample::PlanarBlock;
        use crate::{BLOCK_SIZE, CHANNELS_COUNT};

        let result = panic::catch_unwind(|| {
            let mut block = PlanarBlock::<f32>::default();
            let input: [Vec<f32>; CHANNELS_COUNT] = [
                (0..BLOCK_SIZE).map(|i| i as f32).collect(),
                (0..BLOCK_SIZE).map(|i| i as f32 + 1.0).collect(),
            ];

            // Panic if we try to copy more than the block size
            block.copy_from_planar_vec(&input, 0, BLOCK_SIZE + 1);
        });

        assert!(
            result.is_err(),
            "Expected panic when copying more than block size"
        );
    }

    #[test]
    fn copy_from_test() {
        use crate::sample::PlanarBlock;
        use crate::{BLOCK_SIZE, CHANNELS_COUNT};

        let mut block1 = PlanarBlock::<f32>::default();
        let mut block2 = PlanarBlock::<f32>::default();

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block1.samples[channel][i] = i as f32 + channel as f32;
            }
        }

        block2.copy_from(&block1);

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                assert_eq!(block2.samples[channel][i], i as f32 + channel as f32);
            }
        }
    }

    #[test]
    fn mix_test() {
        use crate::sample::PlanarBlock;
        use crate::{BLOCK_SIZE, CHANNELS_COUNT};

        let mut block1 = PlanarBlock::<f32>::default();
        let mut block2 = PlanarBlock::<f32>::default();

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block1.samples[channel][i] = i as f32 + channel as f32;
                block2.samples[channel][i] = (i as f32 + channel as f32) * 2.0;
            }
        }

        block1.mix(&block2);

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                assert_eq!(
                    block1.samples[channel][i],
                    (i as f32 + channel as f32) * 3.0
                );
            }
        }
    }

    #[bench]
    fn mix_bench(b: &mut Bencher) {
        use crate::sample::PlanarBlock;
        use crate::{BLOCK_SIZE, CHANNELS_COUNT};

        let mut block1 = PlanarBlock::<f32>::default();
        let mut block2 = PlanarBlock::<f32>::default();

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block1.samples[channel][i] = i as f32 + channel as f32;
                block2.samples[channel][i] = (i as f32 + channel as f32) * 2.0;
            }
        }

        b.iter(|| {
            block1.mix(&block2);
        });
    }

    #[test]
    fn gain_clamp_test() {
        use crate::sample::PlanarBlock;
        use crate::{BLOCK_SIZE, CHANNELS_COUNT};

        let mut block = PlanarBlock::<f32>::default();
        let gain = 0.5;

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block.samples[channel][i] = (i as f32 + channel as f32) / 10.0;
            }
        }

        block.pan_gain_phase_clamp(0.0, gain, false);

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                assert_eq!(
                    block.samples[channel][i],
                    ((i as f32 + channel as f32) * gain / 10.0).clamp(-1.0, 1.0)
                );
            }
        }
    }

    #[test]
    fn phase_clamp_test() {
        use crate::sample::PlanarBlock;
        use crate::{BLOCK_SIZE, CHANNELS_COUNT};

        let mut block = PlanarBlock::<f32>::default();
        let invert_phase = true;

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block.samples[channel][i] = (i as f32 + channel as f32) / 10.0;
            }
        }

        block.pan_gain_phase_clamp(0.0, 1.0, invert_phase);

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                assert_eq!(
                    block.samples[channel][i],
                    -((i as f32 + channel as f32) / 10.0).clamp(-1.0, 1.0)
                );
            }
        }
    }

    #[test]
    fn pan_clamp_test() {
        use crate::sample::PlanarBlock;
        use crate::{BLOCK_SIZE, CHANNELS_COUNT};

        let mut block_right = PlanarBlock::<f32>::default();
        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block_right.samples[channel][i] = (i as f32 + channel as f32) / 10.0;
            }
        }

        block_right.pan_gain_phase_clamp(1.0, 1.0, false);
        for i in 0..BLOCK_SIZE {
            assert_eq!(block_right.samples[0][i], 0.0); // Left channel
        }

        let mut block_left = PlanarBlock::<f32>::default();
        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block_left.samples[channel][i] = (i as f32 + channel as f32) / 10.0;
            }
        }
        block_left.pan_gain_phase_clamp(-1.0, 1.0, false);
        for i in 0..BLOCK_SIZE {
            assert_eq!(block_left.samples[1][i], 0.0); // Right channel
        }
    }

    #[bench]
    fn pan_gain_phase_clamp_bench(b: &mut Bencher) {
        use crate::sample::PlanarBlock;
        use crate::{BLOCK_SIZE, CHANNELS_COUNT};

        let mut block = PlanarBlock::<f32>::default();
        let gain = 0.5;

        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                block.samples[channel][i] = (i as f32 + channel as f32) / 10.0;
            }
        }

        b.iter(|| {
            block.pan_gain_phase_clamp(0.0, gain, false);
        });
    }
}
