use crate::entities::events::{Event, EventTarget, EventTargetId};
use crate::entities::{BlockInfo, Effect};
use crate::sample::{PlanarBlock, LEFT_CHANNEL, RIGHT_CHANNEL};
use crate::BLOCK_SIZE;

#[derive(Debug, Clone, PartialEq)]
pub enum LPFEffectEvent {
    Bypass(bool),
    SetCutoff { cutoff: f32, sample_rate: f32 },
}

fn cutoff_to_alpha(cutoff: f32, sample_rate: f32) -> f32 {
    let dt = 1.0 / sample_rate;
    let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
    dt / (rc + dt)
}

pub struct LPFEffect {
    id: EventTargetId,
    bypass: bool,

    y: f32,
    alpha: f32,
}

fn dispatch_lpf(ptr: *mut u8, event: &Event) {
    let lpf: &mut LPFEffect = unsafe { &mut *(ptr as *mut LPFEffect) };
    lpf.dispatch(event);
}

impl LPFEffect {
    pub fn new(cutoff: f32, sample_rate: f32) -> Self {
        Self {
            id: EventTargetId::new(),
            bypass: false,
            y: 0.0,
            alpha: cutoff_to_alpha(cutoff, sample_rate),
        }
    }
    
    pub fn get_id(&self) -> EventTargetId {
        self.id
    }
    
    fn create_event_target(&self) -> EventTarget {
        EventTarget::new(dispatch_lpf, self.id, self)
    }
}

impl Effect for LPFEffect {
    fn get_targets(&self) -> Vec<EventTarget> {
        vec![self.create_event_target()]
    }
    
    fn dispatch(&mut self, event: &Event) {
        match event {
            Event::LPF(LPFEffectEvent::Bypass(bypass)) => {
                self.bypass = *bypass;
            }
            Event::LPF(LPFEffectEvent::SetCutoff { cutoff, sample_rate }) => {
                self.alpha = cutoff_to_alpha(*cutoff, *sample_rate);
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
        // TODO: Support of multi-channel processing
        for i in 0..BLOCK_SIZE {
            let sample = input.samples[LEFT_CHANNEL][i];

            // TODO: Add support for SIMD optimization
            self.y = self.alpha * sample + (1.0 - self.alpha) * self.y;

            output.samples[LEFT_CHANNEL][i] = self.y;
            output.samples[RIGHT_CHANNEL][i] = self.y;
        }
    }
}
