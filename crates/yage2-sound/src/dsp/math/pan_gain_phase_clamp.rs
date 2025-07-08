use crate::sample::{PlanarBlock, LEFT_CHANNEL, RIGHT_CHANNEL};
use crate::{BLOCK_SIZE, CHANNELS_COUNT};

#[inline(never)]
#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
#[target_feature(enable = "neon")]
pub unsafe fn neon_block_m4(block: &mut PlanarBlock<f32>, pan: f32, gain: f32) {
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
        vst1q_f32(
            block.samples[RIGHT_CHANNEL].as_mut_ptr().add(i),
            scaled_right,
        );

        i += 4; // Process 4 samples at a time
    }
}

#[inline(never)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
pub unsafe fn avx512_block_m32(
    block: &mut PlanarBlock<f32>,
    pan: f32,
    gain: f32,
    invert_phase: bool,
) {
    use core::arch::x86_64::*;

    let one = _mm512_set1_ps(1.0);
    let neg_one = _mm512_set1_ps(-1.0);

    macro_rules! process_channel {
        ($gain:expr, $channel_index:expr) => {
            let mul = _mm512_set1_ps($gain);
            let mut i = 0;
            while i < BLOCK_SIZE {
                let in0 = _mm512_loadu_ps(block.samples[$channel_index].as_ptr().add(i));
                let in1 = _mm512_loadu_ps(block.samples[$channel_index].as_ptr().add(i + 16));

                let out0 = _mm512_mul_ps(in0, mul);
                let out0 = _mm512_min_ps(_mm512_max_ps(out0, neg_one), one); // Clamp to [-1.0, 1.0]

                let out1 = _mm512_mul_ps(in1, mul);
                let out1 = _mm512_min_ps(_mm512_max_ps(out1, neg_one), one);

                _mm512_storeu_ps(block.samples[$channel_index].as_mut_ptr().add(i), out0);
                _mm512_storeu_ps(block.samples[$channel_index].as_mut_ptr().add(i + 16), out1);

                i += 32; // Process 32 samples at a time
            }
        };
    }

    // Process left channel
    let left_gain = gain * (1.0 - pan).sqrt() * (if invert_phase { -1.0 } else { 1.0 });
    process_channel!(left_gain, LEFT_CHANNEL);

    // Process right channel
    let right_gain = gain * (1.0 + pan).sqrt() * (if invert_phase { -1.0 } else { 1.0 });
    process_channel!(right_gain, RIGHT_CHANNEL);
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
        };
    }

    // Process left channel
    let left_gain = gain * (1.0 - pan).sqrt() * (if invert_phase { -1.0 } else { 1.0 });
    process_channel!(left_gain, LEFT_CHANNEL);

    // Process right channel
    let right_gain = gain * (1.0 + pan).sqrt() * (if invert_phase { -1.0 } else { 1.0 });
    process_channel!(right_gain, RIGHT_CHANNEL);
}

#[inline(never)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
pub unsafe fn avx_block_m32(block: &mut PlanarBlock<f32>, pan: f32, gain: f32, invert_phase: bool) {
    use core::arch::x86_64::*;
    todo!();
}

#[inline(never)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.2")]
pub unsafe fn sse42_block_m32(
    block: &mut PlanarBlock<f32>,
    pan: f32,
    gain: f32,
    invert_phase: bool,
) {
    use core::arch::x86_64::*;
    todo!();
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
