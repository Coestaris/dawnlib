use crate::entities::events::{AudioEventTarget, AudioEventTargetId, AudioEventType};
use crate::entities::{BlockInfo, Effect};
use crate::sample::PlanarBlock;
use crate::{BLOCK_SIZE, CHANNELS_COUNT};

#[derive(Debug, Clone, PartialEq)]
pub enum SoftClipEffectEvent {
    Bypass(bool),
    SetThreshold(f32),
    SetRatio(f32),
}

fn dispatch_soft_clip(ptr: *mut u8, event: &AudioEventType) {
    let soft_clip: &mut SoftClipEffect = unsafe { &mut *(ptr as *mut SoftClipEffect) };
    soft_clip.dispatch(event);
}

pub struct SoftClipEffect {
    id: AudioEventTargetId,
    bypass: bool,
    threshold: f32,
    ratio: f32,
}

impl SoftClipEffect {
    pub fn new(threshold: f32, ratio: f32) -> Self {
        Self {
            id: AudioEventTargetId::new(),
            bypass: false,
            threshold,
            ratio,
        }
    }

    pub fn get_id(&self) -> AudioEventTargetId {
        self.id
    }

    fn create_event_target(&self) -> AudioEventTarget {
        AudioEventTarget::new(dispatch_soft_clip, self.id, self)
    }
}

impl Effect for SoftClipEffect {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
            AudioEventType::SoftClip(SoftClipEffectEvent::Bypass(bypass)) => {
                self.bypass = *bypass;
            }
            AudioEventType::SoftClip(SoftClipEffectEvent::SetThreshold(threshold)) => {
                self.threshold = *threshold;
            }
            AudioEventType::SoftClip(SoftClipEffectEvent::SetRatio(ratio)) => {
                self.ratio = *ratio;
            }
            _ => {}
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
        for channel in 0..CHANNELS_COUNT {
            for i in 0..BLOCK_SIZE {
                // TODO: SIMD optimization for soft clipping
                let sample = input.samples[channel][i];

                // Apply soft clipping using tanh function
                let gain = 1.5;
                let scaled = sample * gain;
                output.samples[channel][i] = scaled.tanh();
            }
        }
    }
}
