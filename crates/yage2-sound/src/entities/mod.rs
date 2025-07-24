use std::cell::UnsafeCell;
use crate::entities::events::{Event, EventTarget, EventTargetId};
use crate::sample::PlanarBlock;
use crate::{SampleRate, SamplesCount};
use std::collections::HashMap;

pub mod bus;
pub mod effects;
pub mod events;
pub mod sources;

#[repr(C)]
#[derive(Debug)]
pub struct NodeRef<'a, T> {
    ptr: *const T,
    _marker: std::marker::PhantomData<&'a T>,
}

impl<'a, T> NodeRef<'a, T> {
    pub fn to_static(&self) -> NodeRef<'static, T> {
        NodeRef {
            ptr: self.ptr as *const T,
            _marker: std::marker::PhantomData,
        }
    }
}
unsafe impl<'a, T> Send for NodeRef<'a, T> where T: Send {}
unsafe impl<'a, T> Sync for NodeRef<'a, T> where T: Sync {}

impl<'a, T> NodeRef<'a, T> {
    pub fn new(reference: &'a T) -> Self {
        NodeRef {
            ptr: reference as *const T,
            _marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn as_ref(&self) -> &'a T {
        unsafe { &*self.ptr }
    }

    pub(crate) fn as_mut(&self) -> &'a mut T {
        unsafe { &mut *(self.ptr as *mut T) }
    }
}

pub(crate) trait Effect {
    fn get_targets(&self) -> Vec<EventTarget> {
        // Default implementation returns an empty vector
        vec![]
    }
    fn dispatch(&mut self, _event: &Event) {
        // Bypass effect does not handle events
    }

    fn bypass(&self) -> bool;

    fn render(&mut self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>, info: &BlockInfo);
}

pub(crate) struct BlockInfo {
    sample_index: SamplesCount,
    sample_rate: SampleRate,
}

impl BlockInfo {
    pub(crate) fn new(sample_index: SamplesCount, sample_rate: SampleRate) -> Self {
        BlockInfo {
            sample_index,
            sample_rate,
        }
    }

    fn sample_index(&self) -> SamplesCount {
        self.sample_index
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn time(&self, i: SamplesCount) -> f32 {
        (self.sample_index as f32 + i as f32) / self.sample_rate as f32
    }
}

pub(crate) trait Source {
    fn get_targets(&self) -> Vec<EventTarget>;
    fn dispatch(&mut self, event: &Event) {
        // Default implementation does nothing
    }

    fn frame_start(&mut self) {
        // Default implementation does nothing
    }
    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32>;
}

pub struct Sink<T: Source> {
    master: UnsafeCell<T>,
    event_router: HashMap<EventTargetId, EventTarget>,
}
unsafe impl<T: Source> Send for Sink<T> {}
unsafe impl<T: Source> Sync for Sink<T> {}

impl<T: Source> Sink<T> {
    pub fn new(master: T) -> Self {
        let targets = master.get_targets();
        let mut event_router = HashMap::new();
        for target in targets {
            event_router.insert(target.get_id(), target);
        }

        Sink {
            master: UnsafeCell::new(master),
            event_router,
        }
    }

    fn dispatch_event(&self, target_id: EventTargetId, event: &Event) {
        if let Some(target) = self.event_router.get(&target_id) {
            target.dispatch(event);
        }
    }

    pub fn render(&self, info: &BlockInfo) -> &PlanarBlock<f32> {
        // TODO: Process events for the master source

        unsafe {
            let master = &mut *self.master.get();

            // Propagate the frame start to the master source
            master.frame_start();
            // Render the master source
            master.render(info)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::bus::{Bus, BusEvent};
    use crate::entities::effects::bypass::BypassEffect;
    use crate::entities::sources::actor::ActorsSource;
    use crate::entities::sources::multiplexer::{
        Multiplexer1Source, Multiplexer2Source, MultiplexerSourceEvent,
    };
    use log;

    struct SoftClip {}
    impl Effect for SoftClip {
        fn bypass(&self) -> bool {
            todo!()
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

    struct FreeVerb {}
    impl Effect for FreeVerb {
        fn bypass(&self) -> bool {
            todo!()
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

    #[test]
    fn test() {
        // Setup basic logging
        log::set_max_level(log::LevelFilter::Debug);
        struct Logger;
        impl log::Log for Logger {
            fn enabled(&self, metadata: &log::Metadata) -> bool {
                metadata.level() <= log::Level::Debug
            }

            fn log(&self, record: &log::Record) {
                println!("{} - {}", record.level(), record.args());
            }

            fn flush(&self) {}
        }
        static LOGGER: Logger = Logger;
        log::set_logger(&LOGGER).unwrap();

        let actors_effect = BypassEffect {};
        let actors_source = ActorsSource::new();
        let actors_bus = Bus::new(1.0, &actors_effect, &actors_source);

        let freeverb = FreeVerb {};
        let send_source = Multiplexer1Source::new(&actors_bus, 0.5);
        let send_bus = Bus::new(1.0, &freeverb, &send_source);

        let soft_clip = SoftClip {};
        let master_source = Multiplexer2Source::new(&actors_bus, &send_bus, 0.7, 0.3);
        let master_bus = Bus::new(1.0, &soft_clip, &master_source);

        let sink = Sink::new(master_bus);
        assert_ne!(
            sink.event_router.len(),
            0,
            "Event router should not be empty"
        );

        let info = BlockInfo::new(0, 48000);
        let output = sink.render(&info);

        sink.dispatch_event(send_bus.get_id(), &Event::Bus(BusEvent::ChangeGain(0.8)));
        sink.dispatch_event(
            master_source.get_id(),
            &Event::Multiplexer(MultiplexerSourceEvent::ChangeMix(0, 0.6)),
        );
    }
}
