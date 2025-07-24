fn soft_limit(sample: f32) -> f32 {
    let threshold = 0.9;
    if sample.abs() <= threshold {
        sample
    } else {
        let sign = sample.signum();
        let excess = sample.abs() - threshold;
        sign * (threshold + (1.0 - (-excess * 10.0).exp()) / 10.0)
    }
}
