use crate::sample::{InterleavedBlock, PlanarBlock, LEFT_CHANNEL, RIGHT_CHANNEL};
use crate::{BLOCK_SIZE, CHANNELS_COUNT};

#[inline(never)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
pub unsafe fn avx512_block_m32(input: &PlanarBlock<f32>, output: &mut InterleavedBlock<f32>) {
    use core::arch::x86_64::*;

    let mut ch0 = input.samples[LEFT_CHANNEL].as_ptr();
    let mut ch1 = input.samples[RIGHT_CHANNEL].as_ptr();
    let mut out_ptr = output.samples.as_mut_ptr() as *mut f32;

    let indices_1 = _mm512_set_epi32(
        16 + 7,
        7,
        16 + 6,
        6,
        16 + 5,
        5,
        16 + 4,
        4,
        16 + 3,
        3,
        16 + 2,
        2,
        16 + 1,
        1,
        16 + 0,
        0,
    );
    let indices_2 = _mm512_set_epi32(
        16 + 15,
        15,
        16 + 14,
        14,
        16 + 13,
        13,
        16 + 12,
        12,
        16 + 11,
        11,
        16 + 10,
        10,
        16 + 9,
        9,
        16 + 8,
        8,
    );

    let mut i = 0;
    while i < BLOCK_SIZE {
        let a = _mm512_loadu_ps(ch0);
        ch0 = ch0.add(16);
        let b = _mm512_loadu_ps(ch1);
        ch1 = ch1.add(16);

        let interleaved_1 = _mm512_permutex2var_ps(a, indices_1, b);
        let interleaved_2 = _mm512_permutex2var_ps(a, indices_2, b);

        _mm512_storeu_ps(out_ptr, interleaved_1);
        out_ptr = out_ptr.add(16);
        _mm512_storeu_ps(out_ptr, interleaved_2);
        out_ptr = out_ptr.add(16);

        i += 16; // Process 16 samples at a time
    }
}

#[inline(never)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn avx2_block_m32(input: &PlanarBlock<f32>, output: &mut InterleavedBlock<f32>) {
    use core::arch::x86_64::*;

    let mut ch0 = input.samples[LEFT_CHANNEL].as_ptr();
    let mut ch1 = input.samples[RIGHT_CHANNEL].as_ptr();
    let mut out_ptr = output.samples.as_mut_ptr() as *mut f32;

    let mut i = 0;
    while i < BLOCK_SIZE {
        // a.0, a.1, a.2, a.3, a.4, a.5, a.6, a.7
        let a = _mm256_loadu_ps(ch0);
        ch0 = ch0.add(8);

        // b.0, b.1, b.2, b.3, b.4, b.5, b.6, b.7
        let b = _mm256_loadu_ps(ch1);
        ch1 = ch1.add(8);

        // a0 b0 a1 b1 | a4 b4 a5 b5
        let lo = _mm256_unpacklo_ps(a, b);
        // a2 b2 a3 b3 | a6 b6 a7 b7
        let hi = _mm256_unpackhi_ps(a, b);

        // a0 b0 a1 b1 | a2 b2 a3 b3
        let interleaved_1 = _mm256_permute2f128_ps::<0b00100000>(lo, hi);
        // a4 b4 a5 b5 | a6 b6 a7 b7
        let interleaved_2 = _mm256_permute2f128_ps::<0b00110001>(lo, hi);

        _mm256_storeu_ps(out_ptr, interleaved_1);
        out_ptr = out_ptr.add(8);
        _mm256_storeu_ps(out_ptr, interleaved_2);
        out_ptr = out_ptr.add(8);

        i += 8; // Process 8 samples at a time
    }
}

#[inline(never)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
pub unsafe fn avx_block_m32(input: &PlanarBlock<f32>, output: &mut InterleavedBlock<f32>) {
    todo!();
}

#[inline(never)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.2")]
pub unsafe fn sse42_block_m32(input: &PlanarBlock<f32>, output: &mut InterleavedBlock<f32>) {
    use core::arch::x86_64::*;
    todo!();
}

#[inline(never)]
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
pub unsafe fn neon_block_m4(input: &PlanarBlock<f32>, output: &mut InterleavedBlock<f32>) {
    use core::arch::aarch64::*;

    let mut ch0 = input.samples[LEFT_CHANNEL].as_ptr();
    let mut ch1 = input.samples[RIGHT_CHANNEL].as_ptr();
    let mut out_ptr = output.samples.as_mut_ptr() as *mut f32;

    let mut i = 0;
    while i < BLOCK_SIZE {
        let a = vld1q_f32(ch0);
        ch0 = ch0.add(4);
        let b = vld1q_f32(ch1);
        ch1 = ch1.add(4);

        let interleaved_1 = vzip1q_f32(a, b);
        let interleaved_2 = vzip2q_f32(a, b);

        vst1q_f32(out_ptr, interleaved_1);
        out_ptr = out_ptr.add(4);
        vst1q_f32(out_ptr, interleaved_2);
        out_ptr = out_ptr.add(4);

        i += 8; // Process 8 samples at a time
    }
}

#[inline(never)]
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "sve")]
pub(crate) fn sve_block_m4(input: &PlanarBlock<f32>, output: &mut InterleavedBlock<f32>) {
    todo!()
}

#[inline(always)]
pub(crate) fn fallback(input: &PlanarBlock<f32>, output: &mut InterleavedBlock<f32>) {
    for i in 0..BLOCK_SIZE {
        for channel in 0..CHANNELS_COUNT {
            let sample = input.samples[channel][i];
            output.samples[i].channels[channel] = sample;
        }
    }
}
