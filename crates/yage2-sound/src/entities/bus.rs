use crate::entities::multiplexers::MultiplexerSource;
use crate::entities::{Effect, Event, EventTarget, EventTargetId, NodeRef, Source};
use crate::sample::PlanarBlock;
use log::debug;

pub struct Bus<'a, E, S>
where
    E: Effect,
    S: Source,
{
    id: EventTargetId,
    gain: f32,
    effect: NodeRef<'a, E>,
    source: NodeRef<'a, S>,
    output: PlanarBlock<f32>,
}

fn dispatch_bus<E, S>(ptr: *mut u8, event: &Event)
where
    E: Effect,
    S: Source,
{
    let bus: &mut Bus<E, S> = unsafe { &mut *(ptr as *mut Bus<E, S>) };
    bus.dispatch(event);
}

impl<E, S> Bus<'_, E, S>
where
    E: Effect,
    S: Source,
{
    pub fn new(gain: f32, effect: &E, source: &S) -> Self {
        Bus {
            id: EventTargetId::new(),
            gain,
            effect: NodeRef::new(unsafe { &*(effect as *const E) }),
            source: NodeRef::new(unsafe { &*(source as *const S) }),
            output: PlanarBlock::default(),
        }
    }

    pub fn get_id(&self) -> EventTargetId {
        self.id
    }

    fn create_event_target(&self) -> EventTarget {
        EventTarget {
            id: self.id,
            dispatcher: dispatch_bus::<E, S>,
            ptr: self as *const _ as *mut u8,
        }
    }
}

impl<E, S> Source for Bus<'_, E, S>
where
    E: Effect,
    S: Source,
{
    fn get_targets(&self) -> Vec<EventTarget> {
        let mut targets = self.source.as_ref().get_targets();
        targets.extend(self.effect.as_ref().get_targets());
        targets.push(self.create_event_target());
        targets
    }

    fn frame_start(&mut self) {
        self.source.as_mut().frame_start();
    }

    fn dispatch(&mut self, event: &Event) {
        match event {
            Event::ChangeBusGain(gain) => {
                debug!("ChangeBusGain of bus {} to {}", self.id, self.gain);
                self.gain = *gain;
            }

            _ => {}
        }
    }

    fn render(&mut self) -> &PlanarBlock<f32> {
        // Render the source
        let input = self.source.as_mut().render();

        // Apply the effect
        let effect = self.effect.as_mut();
        if !effect.bypass() {
            effect.process(input, &mut self.output);
        } else {
            self.output.copy_from(input);
        }

        // Apply gain
        for channel in 0..self.output.samples.len() {
            for sample in &mut self.output.samples[channel] {
                *sample *= self.gain;
            }
        }

        &self.output
    }
}
