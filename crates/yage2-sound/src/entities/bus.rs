use crate::entities::{BlockInfo, Effect, Event, EventTarget, EventTargetId, NodeRef, Source};
use crate::sample::PlanarBlock;
use log::debug;

pub enum BusEvent {
    ChangeGain(f32),
}

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

impl<'a, E, S> Bus<'a, E, S>
where
    E: Effect,
    S: Source,
{
    pub fn new(gain: f32, effect: &'a E, source: &'a S) -> Self {
        Bus {
            id: EventTargetId::new(),
            gain,
            effect: NodeRef::<'a>::new(effect),
            source: NodeRef::<'a>::new(source),
            output: PlanarBlock::default(),
        }
    }

    pub fn get_id(&self) -> EventTargetId {
        self.id
    }

    fn create_event_target(&self) -> EventTarget {
        EventTarget::new(dispatch_bus::<E, S>, self.id, self)
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

    fn dispatch(&mut self, event: &Event) {
        match event {
            Event::Bus(BusEvent::ChangeGain(gain)) => {
                debug!("ChangeBusGain of bus {} to {}", self.id, self.gain);
                self.gain = *gain;
            }

            _ => {}
        }
    }

    fn frame_start(&mut self) {
        self.source.as_mut().frame_start();
    }

    #[inline(always)]
    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32> {
        // Render the source
        let input = self.source.as_mut().render(info);

        // Apply the effect
        let effect = self.effect.as_mut();
        if !effect.bypass() {
            effect.render(input, &mut self.output, info);
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
