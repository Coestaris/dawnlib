use crate::entities::{BlockInfo, Effect, Event, EventTarget, EventTargetId, NodeRef, Source};
use crate::sample::PlanarBlock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusEvent {}

pub struct Bus<'a, E, S>
where
    E: Effect,
    S: Source,
{
    id: EventTargetId,
    cached: bool,
    gain: f32,
    pan: f32,
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
    pub fn new(effect: &'a E, source: &'a S, gain: Option<f32>, pan: Option<f32>) -> Self {
        Bus {
            id: EventTargetId::new(),
            effect: NodeRef::<'a>::new(effect),
            source: NodeRef::<'a>::new(source),
            output: PlanarBlock::default(),
            gain: gain.unwrap_or(1.0),
            pan: pan.unwrap_or(0.0),
            cached: false,
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
        self.cached = false;
    }

    #[inline(always)]
    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32> {
        if self.cached {
            return &self.output;
        }

        // Render the source
        let input = self.source.as_mut().render(info);

        // Apply the effect
        let effect = self.effect.as_mut();
        if !effect.bypass() {
            effect.render(input, &mut self.output, info);
        } else {
            self.output.copy_from(input);
        }

        // Apply gain and pan
        self.output.gain_pan(self.gain, self.pan);

        self.cached = true;
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
    use crate::sample::{LEFT_CHANNEL, RIGHT_CHANNEL};

    #[test]
    fn test_bus() {
        detect_features();

        let effect = BypassEffect::new();
        let source = TestSource::new();
        let mut bus = Bus::new(&effect, &source, None, None);

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

    #[test]
    fn test_bus_gain() {
        detect_features();

        let effect = BypassEffect::new();
        let source = TestSource::new();
        let mut bus = Bus::new(&effect, &source, Some(0.5), None);

        for i in 0..10 {
            bus.frame_start();
            let info = BlockInfo::new(0, 44_100);
            let output = bus.render(&info);

            // Result should be half of the TestSource output (0.5, 1.0, 1.5, ...)
            for i in 0..output.samples[0].len() {
                for channel in 0..output.samples.len() {
                    assert_eq!(output.samples[channel][i], (i + 1) as f32 * 0.5);
                }
            }
        }
    }

    #[test]
    fn test_bus_pan() {
        detect_features();

        let effect = BypassEffect::new();
        let source = TestSource::new();
        let mut bus = Bus::new(&effect, &source, None, Some(1.0));

        bus.frame_start();
        let info = BlockInfo::new(0, 44_100);
        let output = bus.render(&info);

        // Result should be the same as output of the TestSource (1,2,3,...)
        for i in 0..output.samples[0].len() {
            assert_eq!(output.samples[LEFT_CHANNEL][i], 0.0);
        }

        let mut bus = Bus::new(&effect, &source, None, Some(-1.0));

        bus.frame_start();
        let info = BlockInfo::new(0, 44_100);
        let output = bus.render(&info);

        // Result should be the same as output of the TestSource (1,2,3,...)
        for i in 0..output.samples[0].len() {
            assert_eq!(output.samples[RIGHT_CHANNEL][i], 0.0);
        }
    }

    #[bench]
    fn bench_bus(b: &mut test::Bencher) {
        detect_features();

        let effect = BypassEffect::new();
        let source = TestSource::new();
        let mut bus = Bus::new(&effect, &source, None, None);
        let info = BlockInfo::new(0, 44_100);

        b.iter(|| {
            bus.frame_start();
            bus.render(&info);
        });
    }
}
