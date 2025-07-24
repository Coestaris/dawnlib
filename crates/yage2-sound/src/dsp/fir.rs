pub struct FirFilter<const N: usize> {
    coeffs: [f32; N],
    buffer: [f32; N],
    pos: usize,
}

impl<const N: usize> FirFilter<N> {
    pub fn new(coeffs: [f32; N]) -> Self {
        Self {
            coeffs,
            buffer: [0.0; N],
            pos: 0,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.buffer[self.pos] = input;
        let mut acc = 0.0;
        for i in 0..N {
            let index = (self.pos + N - i) % N;
            acc += self.buffer[index] * self.coeffs[i];
        }
        self.pos = (self.pos + 1) % N;
        acc
    }
}

// Чем больше — тем круче спад и уже полоса, но дороже
// Обычно 32–128 — норм
fn design_lpf(f_c: f32, sample_rate: f32, taps: usize) -> Vec<f32> {
    let norm_cutoff = f_c / sample_rate; // нормализованная частота
    let mut coeffs = Vec::with_capacity(taps);

    for i in 0..taps {
        let n = i as f32 - (taps as f32 - 1.0) / 2.0;
        let sinc = if n == 0.0 {
            1.0
        } else {
            (2.0 * std::f32::consts::PI * norm_cutoff * n).sin() / (std::f32::consts::PI * n)
        };

        // Hamming window
        let window =
            0.54 - 0.46 * ((2.0 * std::f32::consts::PI * i as f32) / (taps as f32 - 1.0)).cos();

        coeffs.push(2.0 * norm_cutoff * sinc * window);
    }

    coeffs
}


pub struct SimpleLPF {
    pub y: f32, // внутреннее состояние
    pub alpha: f32,
}

impl SimpleLPF {
    pub fn new(cutoff: f32, sample_rate: f32) -> Self {
        let dt = 1.0 / sample_rate;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let alpha = dt / (rc + dt);
        Self { y: 0.0, alpha }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        self.y = self.alpha * x + (1.0 - self.alpha) * self.y;
        self.y
    }
}
