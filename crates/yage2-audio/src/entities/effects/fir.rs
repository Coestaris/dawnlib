use crate::entities::events::{AudioEventTarget, AudioEventTargetId, AudioEventType};
use crate::entities::{BlockInfo, Effect};
use crate::sample::{PlanarBlock, LEFT_CHANNEL, RIGHT_CHANNEL};
use crate::{BLOCK_SIZE, CHANNELS_COUNT};

#[derive(Debug, Clone, PartialEq)]
pub enum FirFilterEffectEvent {
    Bypass(bool),
}

pub struct FirFilterEffect<const N: usize> {
    id: AudioEventTargetId,
    bypass: bool,

    coeffs: [f32; N],
    buffer: [f32; N],
    pos: usize,
}

fn dispatch_fir_filter<const N: usize>(ptr: *mut u8, event: &AudioEventType) {
    let fir_filter: &mut FirFilterEffect<N> = unsafe { &mut *(ptr as *mut FirFilterEffect<N>) };
    fir_filter.dispatch(event);
}

impl<const N: usize> FirFilterEffect<N> {
    /// More taps - the steeper the filter, narrower the passband
    /// but CPU usage increases.
    /// Usually, N = 32 or N = 64 is enough for most applications.
    pub fn new_from_design(f_c: f32, sample_rate: f32) -> Self {
        // Normalized cutoff frequency
        let norm_cutoff = f_c / sample_rate;
        let mut coeffs = [0.0; N];

        for i in 0..N {
            let n = i as f32 - (N as f32 - 1.0) / 2.0;
            let sinc = if n == 0.0 {
                1.0
            } else {
                (2.0 * std::f32::consts::PI * norm_cutoff * n).sin() / (std::f32::consts::PI * n)
            };

            // Hamming window
            let window =
                0.54 - 0.46 * ((2.0 * std::f32::consts::PI * i as f32) / (N as f32 - 1.0)).cos();

            coeffs[i] = 2.0 * norm_cutoff * sinc * window;
        }

        // Normalize coefficients
        let sum: f32 = coeffs.iter().sum();
        for i in 0..N {
            coeffs[i] /= sum;
        }

        FirFilterEffect::new(coeffs)
    }

    pub fn new(coeffs: [f32; N]) -> Self {
        Self {
            id: AudioEventTargetId::new(),
            bypass: false,
            coeffs,
            buffer: [0.0; N],
            pos: 0,
        }
    }

    pub fn get_id(&self) -> AudioEventTargetId {
        self.id
    }

    fn create_event_target(&self) -> AudioEventTarget {
        AudioEventTarget::new(dispatch_fir_filter::<N>, self.id, self)
    }
}

impl<const N: usize> Effect for FirFilterEffect<N> {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
            AudioEventType::FirFilter(FirFilterEffectEvent::Bypass(bypass)) => {
                self.bypass = *bypass;
            }
            _ => {
                // Handle other events if needed
            }
        }
    }

    fn bypass(&self) -> bool {
        self.bypass
    }

    fn render(
        &mut self,
        input: &PlanarBlock<f32>,
        output: &mut PlanarBlock<f32>,
        info: &BlockInfo,
    ) {
        // TODO: Support of multi-channel processing
        for i in 0..BLOCK_SIZE {
            let sample = input.samples[LEFT_CHANNEL][i];

            // TODO: Add support for SIMD processing
            self.buffer[self.pos] = sample;
            let mut acc = 0.0;
            for i in 0..N {
                let index = (self.pos + N - i) % N;
                acc += self.buffer[index] * self.coeffs[i];
            }
            self.pos = (self.pos + 1) % N;

            output.samples[LEFT_CHANNEL][i] = acc;
            output.samples[RIGHT_CHANNEL][i] = acc;
        }
    }
}
