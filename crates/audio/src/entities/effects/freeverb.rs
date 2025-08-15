use crate::entities::events::{AudioEventTarget, AudioEventTargetId, AudioEventType};
use crate::entities::{BlockInfo, Effect};
use crate::sample::PlanarBlock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltInTuning {
    TuningA,
    TuningB,
    TuningC,
}

impl BuiltInTuning {
    fn apply(&self, freeverb: &mut FreeverbEffect) {
        match self {
            BuiltInTuning::TuningA => {
                freeverb.room_size = 0.5;
                freeverb.damping = 0.5;
                freeverb.wet_level = 0.33;
                freeverb.dry_level = 0.4;
                freeverb.width = 1.0;
                freeverb.freeze_mode = false;
            }
            BuiltInTuning::TuningB => {
                freeverb.room_size = 0.7;
                freeverb.damping = 0.3;
                freeverb.wet_level = 0.5;
                freeverb.dry_level = 0.5;
                freeverb.width = 0.8;
                freeverb.freeze_mode = true;
            }
            BuiltInTuning::TuningC => {
                freeverb.room_size = 0.9;
                freeverb.damping = 0.2;
                freeverb.wet_level = 0.7;
                freeverb.dry_level = 0.3;
                freeverb.width = 0.6;
                freeverb.freeze_mode = false;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FreeverbEffectEvent {
    Bypass(bool),
    SetRoomSize(f32),
    SetDamping(f32),
    SetWetLevel(f32),
    SetDryLevel(f32),
    SetWidth(f32),
    SetFreezeMode(bool),
    SetBuiltInTuning(BuiltInTuning),
}

#[derive(Default)]
pub struct FreeverbEffect {
    id: AudioEventTargetId,
    bypass: bool,

    room_size: f32,
    damping: f32,
    wet_level: f32,
    dry_level: f32,
    width: f32,
    freeze_mode: bool,
}

fn dispatch_freeverb(ptr: *mut u8, event: &AudioEventType) {
    let freeverb: &mut FreeverbEffect = unsafe { &mut *(ptr as *mut FreeverbEffect) };
    freeverb.dispatch(event);
}

impl FreeverbEffect {
    pub fn new_from_tuning(tuning: BuiltInTuning) -> Self {
        let mut freeverb = FreeverbEffect {
            id: AudioEventTargetId::new(),
            ..Default::default()
        };
        tuning.apply(&mut freeverb);
        freeverb
    }

    pub fn get_id(&self) -> AudioEventTargetId {
        self.id
    }

    fn create_event_target(&self) -> AudioEventTarget {
        AudioEventTarget::new(dispatch_freeverb, self.id, self)
    }
}

impl Effect for FreeverbEffect {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
            AudioEventType::Freeverb(FreeverbEffectEvent::Bypass(bypass)) => self.bypass = *bypass,
            AudioEventType::Freeverb(FreeverbEffectEvent::SetRoomSize(size)) => {
                self.room_size = size.clamp(0.0, 1.0)
            }
            AudioEventType::Freeverb(FreeverbEffectEvent::SetDamping(damping)) => {
                self.damping = damping.clamp(0.0, 1.0)
            }
            AudioEventType::Freeverb(FreeverbEffectEvent::SetWetLevel(level)) => {
                self.wet_level = level.clamp(0.0, 1.0)
            }
            AudioEventType::Freeverb(FreeverbEffectEvent::SetDryLevel(level)) => {
                self.dry_level = level.clamp(0.0, 1.0)
            }
            AudioEventType::Freeverb(FreeverbEffectEvent::SetWidth(width)) => {
                self.width = width.clamp(0.0, 1.0)
            }
            AudioEventType::Freeverb(FreeverbEffectEvent::SetFreezeMode(freeze)) => {
                self.freeze_mode = *freeze
            }
            AudioEventType::Freeverb(FreeverbEffectEvent::SetBuiltInTuning(tuning)) => {
                tuning.apply(self);
            }
            _ => {
                // Ignore other events
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
        todo!()
    }
}
