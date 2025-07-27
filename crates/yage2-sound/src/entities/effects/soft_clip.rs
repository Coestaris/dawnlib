use crate::entities::events::{Event, EventTarget, EventTargetId};

#[derive(Debug, Clone, PartialEq)]
pub enum SoftClipEffectEvent {
    Bypass(bool),
    SetThreshold(f32),
    SetRatio(f32),
}

fn dispatch_soft_clip(ptr: *mut u8, event: &Event) {
    let soft_clip: &mut SoftClipEffect = unsafe { &mut *(ptr as *mut SoftClipEffect) };
    soft_clip.dispatch(event);
}

pub struct SoftClipEffect {
    id: EventTargetId,
    bypass: bool,
    threshold: f32,
    ratio: f32,
}

impl SoftClipEffect {
    pub fn new(threshold: f32, ratio: f32) -> Self {
        Self {
            id: EventTargetId::new(),
            bypass: false,
            threshold,
            ratio,
        }
    }
    
    pub fn get_id(&self) -> EventTargetId {
        self.id
    }

    fn create_event_target(&self) -> EventTarget {
        EventTarget::new(dispatch_soft_clip, self.id, self)
    }
}

impl SoftClipEffect {
    fn dispatch(&mut self, event: &Event) {
        match event {
            Event::SoftClip(SoftClipEffectEvent::Bypass(bypass)) => {
                self.bypass = *bypass;
            }
            Event::SoftClip(SoftClipEffectEvent::SetThreshold(threshold)) => {
                self.threshold = *threshold;
            }
            Event::SoftClip(SoftClipEffectEvent::SetRatio(ratio)) => {
                self.ratio = *ratio;
            }
            _ => {}
        }
    }

    fn bypass(&self) -> bool {
        self.bypass
    }
}