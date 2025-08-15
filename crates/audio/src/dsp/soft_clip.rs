// #[inline(always)]
// pub unsafe fn soft_clip_avx(x: __m256) -> __m256 {
//     // y = tanh(1.5 * x)
//     let gain = _mm256_set1_ps(1.5);
//     let scaled = _mm256_mul_ps(x, gain);
//     tanh_ps_avx(scaled)
// }

// tanh(x) ≈ x * (27 + x^2) / (27 + 9x^2)
// Быстрое приближение, приемлемое качество
//
// #[inline(always)]
// unsafe fn tanh_ps_avx(x: __m256) -> __m256 {
//     let x2 = _mm256_mul_ps(x, x);
//     let num = _mm256_add_ps(_mm256_set1_ps(27.0), x2);
//     let denom = _mm256_add_ps(_mm256_set1_ps(27.0), _mm256_mul_ps(x2, _mm256_set1_ps(9.0)));
//     _mm256_mul_ps(x, _mm256_div_ps(num, denom))
// }

fn fallback(input: &mut [f32]) {
    for sample in input.iter_mut() {
        // y = tanh(1.5 * x)
        let gain = 1.5;
        let scaled = *sample * gain;
        *sample = scaled.tanh();
    }
}
