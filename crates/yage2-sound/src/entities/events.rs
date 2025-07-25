use crate::entities::bus::BusEvent;
use crate::entities::sources::actor::ActorsSourceEvent;
use crate::entities::sources::multiplexer::MultiplexerSourceEvent;
use crate::entities::sources::waveform::WaveformSourceEvent;

#[derive(Debug, Clone, PartialEq)]
pub struct EventBox {
    target_id: EventTargetId,
    event: Event,
}

impl EventBox {
    pub fn new(target_id: EventTargetId, event: Event) -> Self {
        EventBox { target_id, event }
    }

    #[inline(always)]
    pub fn get_target_id(&self) -> EventTargetId {
        self.target_id
    }

    #[inline(always)]
    pub fn get_event(&self) -> &Event {
        &self.event
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    // General events
    Mute,
    Unmute,
    Bus(BusEvent),

    // Sources events
    Waveform(WaveformSourceEvent),
    Actors(ActorsSourceEvent),
    Multiplexer(MultiplexerSourceEvent),
    
    #[cfg(test)]
    Test(crate::entities::sources::TestSourceEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventTargetId(usize);

impl std::fmt::Display for EventTargetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventTargetId({})", self.0)
    }
}

impl EventTargetId {
    pub(crate) fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        EventTargetId(id)
    }
}

pub(crate) type EventDispatcher = fn(*mut u8, &Event);

pub struct EventTarget {
    dispatcher: EventDispatcher,
    id: EventTargetId,
    ptr: *mut u8,
}

unsafe impl Send for EventTarget {}
unsafe impl Sync for EventTarget {}

impl EventTarget {
    pub(crate) fn new<T>(dispatcher: EventDispatcher, id: EventTargetId, ptr: &T) -> Self {
        EventTarget {
            dispatcher,
            id,
            ptr: ptr as *const T as *mut u8,
        }
    }

    pub(crate) fn get_id(&self) -> EventTargetId {
        self.id
    }

    #[inline(always)]
    pub(crate) fn dispatch(&self, event: &Event) {
        (self.dispatcher)(self.ptr, event);
    }
}
