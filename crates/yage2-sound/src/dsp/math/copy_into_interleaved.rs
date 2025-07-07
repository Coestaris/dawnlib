use crate::sample::{
    InterleavedBlock, InterleavedSample, PlanarBlock, LEFT_CHANNEL, RIGHT_CHANNEL,
};
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
    todo!();
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

#[inline(always)]
pub(crate) fn fallback(input: &PlanarBlock<f32>, output: &mut InterleavedBlock<f32>) {
    for i in 0..BLOCK_SIZE {
        for channel in 0..CHANNELS_COUNT {
            let sample = input.samples[channel][i];
            output.samples[i].channels[channel] = sample;
        }
    }
}
