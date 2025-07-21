use crate::sample::PlanarBlock;
use std::any::Any;
use std::collections::HashMap;

mod actor;
mod bus;
mod multiplexers;

#[repr(C)]
pub(crate) struct NodeRef<'a, T> {
    ptr: *const T,
    _marker: std::marker::PhantomData<&'a T>,
}

impl<'a, T> NodeRef<'a, T> {
    pub(crate) fn new(reference: &'a T) -> Self {
        NodeRef {
            ptr: reference as *const T,
            _marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn as_ref(&self) -> &'a T {
        unsafe { &*self.ptr }
    }

    pub(crate) fn as_mut(&mut self) -> &'a mut T {
        unsafe { &mut *(self.ptr as *mut T) }
    }
}

enum Event {
    ChangeBusGain(f32),
    ChangeMultiplexerSourceMix(usize, f32), // (index, mix)
    AddActor { id: usize, gain: f32 },
    RemoveActors(usize),
    ChangeListenerPosition(),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EventTargetId(usize);

impl std::fmt::Display for EventTargetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventTargetId({})", self.0)
    }
}

impl EventTargetId {
    pub fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        EventTargetId(id)
    }
}

type EventDispatcher = fn(*mut u8, &Event);

struct EventTarget {
    dispatcher: EventDispatcher,
    id: EventTargetId,
    ptr: *mut u8,
}

trait Effect {
    fn get_targets(&self) -> Vec<EventTarget> {
        // Default implementation returns an empty vector
        vec![]
    }
    fn dispatch(&mut self, _event: &Event) {
        // Bypass effect does not handle events
    }

    fn bypass(&self) -> bool;

    fn process(&mut self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>);
}

struct BypassEffect {}

impl Effect for BypassEffect {
    fn bypass(&self) -> bool {
        true
    }

    fn process(&mut self, _: &PlanarBlock<f32>, _: &mut PlanarBlock<f32>) {
        unreachable!()
    }
}

trait Source {
    fn get_targets(&self) -> Vec<EventTarget>;
    fn dispatch(&mut self, event: &Event) {
        // Default implementation does nothing
    }

    fn frame_start(&mut self) {
        // Default implementation does nothing
    }
    fn render(&mut self) -> &PlanarBlock<f32>;
}

struct Sink<'a, T: Source> {
    master: NodeRef<'a, T>,
    event_router: HashMap<EventTargetId, EventTarget>,
}

impl<'a, T: Source> Sink<'a, T> {
    pub fn new(master: &'a T) -> Self {
        let targets = master.get_targets();
        let mut event_router = HashMap::new();
        for target in targets {
            event_router.insert(target.id, target);
        }

        Sink {
            master: NodeRef::new(master),
            event_router,
        }
    }

    fn dispatch_event(&self, target_id: EventTargetId, event: &Event) {
        if let Some(target) = self.event_router.get(&target_id) {
            (target.dispatcher)(target.ptr, event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::actor::ActorsSource;
    use crate::entities::bus::Bus;
    use crate::entities::multiplexers::{Multiplexer1Source, Multiplexer2Source};
    use log;

    struct SoftClip {}
    impl Effect for SoftClip {
        fn bypass(&self) -> bool {
            todo!()
        }

        fn process(&mut self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>) {
            todo!()
        }
    }

    struct FreeVerb {}
    impl Effect for FreeVerb {
        fn bypass(&self) -> bool {
            todo!()
        }

        fn process(&mut self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>) {
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

        let sink = Sink::new(&master_bus);
        assert_ne!(
            sink.event_router.len(),
            0,
            "Event router should not be empty"
        );

        sink.dispatch_event(send_bus.get_id(), &Event::ChangeBusGain(0.8));
        sink.dispatch_event(
            master_source.get_id(),
            &Event::ChangeMultiplexerSourceMix(0, 0.6),
        );
    }
}
