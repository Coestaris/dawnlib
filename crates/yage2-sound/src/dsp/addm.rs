use crate::sample::PlanarBlock;
use crate::{BLOCK_SIZE, CHANNELS_COUNT};

#[inline(never)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
pub unsafe fn avx512_block_m32(input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, k: f32) {
    use core::arch::x86_64::*;

    let mul = _mm512_set1_ps(k);
    for channel in 0..CHANNELS_COUNT {
        let input_ptr = input.samples.get_unchecked(channel).as_ptr();
        let output_ptr = output.samples.get_unchecked_mut(channel).as_mut_ptr();

        let mut i = 0;
        while i < BLOCK_SIZE {
            let in0 = _mm512_load_ps(input_ptr.add(i));
            let in1 = _mm512_load_ps(input_ptr.add(i + 16));
            let in0 = _mm512_mul_ps(in0, mul);
            let in1 = _mm512_mul_ps(in1, mul);

            let out0 = _mm512_load_ps(output_ptr.add(i));
            let out1 = _mm512_load_ps(output_ptr.add(i + 16));

            _mm512_store_ps(output_ptr.add(i), _mm512_add_ps(in0, out0));
            _mm512_store_ps(output_ptr.add(i + 16), _mm512_add_ps(in1, out1));

            i += 32;
        }
    }
}

#[inline(never)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
// Note, that block adrress MUST be aligned to 32 bytes
//       and BLOCK_SIZE must be a multiple of 32
pub unsafe fn avx2_block_m32(input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, k: f32) {
    // AVX2 intrinsics for x86_64 architecture
    use core::arch::x86_64::*;

    let mul = _mm256_set1_ps(k);
    for channel in 0..CHANNELS_COUNT {
        let input_ptr = input.samples.get_unchecked(channel).as_ptr();
        let output_ptr = output.samples.get_unchecked_mut(channel).as_mut_ptr();

        // 128 floats / 8 = 16 vectors
        // Unroll x4: 16 / 4 = 4 iterations
        let mut i = 0;
        while i < BLOCK_SIZE {
            let in0 = _mm256_load_ps(input_ptr.add(i));
            let in1 = _mm256_load_ps(input_ptr.add(i + 8));
            let in2 = _mm256_load_ps(input_ptr.add(i + 16));
            let in3 = _mm256_load_ps(input_ptr.add(i + 24));

            let in0 = _mm256_mul_ps(in0, mul);
            let in1 = _mm256_mul_ps(in1, mul);
            let in2 = _mm256_mul_ps(in2, mul);
            let in3 = _mm256_mul_ps(in3, mul);

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
pub unsafe fn neon_block_m4(input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32, k: f32>) {
    // NEON intrinsics for ARM architecture
    use core::arch::aarch64::*;

    todo!();
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

#[inline(never)]
#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
#[target_feature(enable = "sve")]
pub unsafe fn sve_block_m4(input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, k: f32) {
    todo!()
}

#[inline(never)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
pub unsafe fn avx_block_m32(input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, k: f32) {
    use core::arch::x86_64::*;
    todo!();
}

#[inline(never)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.2")]
pub unsafe fn sse42_block_m32(input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, k: f32) {
    use core::arch::x86_64::*;

    let mul = _mm_set1_ps(k);
    for channel in 0..CHANNELS_COUNT {
        let input_ptr = input.samples.get_unchecked(channel).as_ptr();
        let output_ptr = output.samples.get_unchecked_mut(channel).as_mut_ptr();

        let mut i = 0;
        while i < BLOCK_SIZE {
            let in0 = _mm_load_ps(input_ptr.add(i));
            let in1 = _mm_load_ps(input_ptr.add(i + 4));

            let in0 = _mm_mul_ps(in0, mul);
            let in1 = _mm_mul_ps(in1, mul);

            let out0 = _mm_load_ps(output_ptr.add(i));
            let out1 = _mm_load_ps(output_ptr.add(i + 4));

            _mm_store_ps(output_ptr.add(i), _mm_add_ps(in0, out0));
            _mm_store_ps(output_ptr.add(i + 4), _mm_add_ps(in1, out1));

            i += 8; // Process 8 samples at a time
        }
    }
}

#[inline(always)]
pub fn fallback(input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, k: f32) {
    for channel in 0..CHANNELS_COUNT as usize {
        for i in 0..BLOCK_SIZE {
            output.samples[channel][i] += input.samples[channel][i] * k;
        }
    }
}
