use crate::sample::PlanarBlock;
use crate::BLOCK_SIZE;

// TODO: Implement SIMD support for the Bus
impl PlanarBlock<f32> {
    pub(crate) fn copy_from(&mut self, input: &PlanarBlock<f32>) {
        unsafe {
            let src = input.samples.as_ptr();
            let dst = self.samples.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, input.samples.len() * BLOCK_SIZE);
        }
    }

    pub(crate) fn mix(&mut self, input: &PlanarBlock<f32>) {
        for channel in 0..self.samples.len() {
            for i in 0..self.samples[channel].len() {
                self.samples[channel][i] += input.samples[channel][i];
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
