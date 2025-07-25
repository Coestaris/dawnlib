use crate::entities::{BlockInfo, Effect, Event, EventTarget, EventTargetId, NodeRef, Source};
use crate::sample::PlanarBlock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusEvent {
}

pub struct Bus<'a, E, S>
where
    E: Effect,
    S: Source,
{
    id: EventTargetId,
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
    pub fn new(effect: &'a E, source: &'a S) -> Self {
        Bus {
            id: EventTargetId::new(),
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
            _ => {}
        }
    }

    #[inline(always)]
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

        &self.output
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use super::*;
    use crate::dsp::detect_features;
    use crate::entities::effects::bypass::BypassEffect;
    use crate::entities::sources::TestSource;

    #[test]
    fn test_bus() {
        detect_features();

        let effect = BypassEffect::new();
        let source = TestSource::new();
        let mut bus = Bus::new(&effect, &source);

        for i in 0..10 {
            bus.frame_start();
            let info = BlockInfo::new(0, 44_100);
            let output = bus.render(&info);

            // Result should be the same as output of the TestSource (1,2,3,...)
            for i in 0..output.samples[0].len() {
                for channel in 0..output.samples.len() {
                    assert_eq!(output.samples[channel][i], (i + 1) as f32);
                }
            }
        }
    }

    #[bench]
    fn bench_bus(b: &mut test::Bencher) {
        detect_features();

        let effect = BypassEffect::new();
        let source = TestSource::new();
        let mut bus = Bus::new(&effect, &source);
        let info = BlockInfo::new(0, 44_100);

        b.iter(|| {
            bus.frame_start();
            bus.render(&info);
        });
    }
}
