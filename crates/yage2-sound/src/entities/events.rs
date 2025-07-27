use crate::entities::bus::BusEvent;
use crate::entities::effects::fir::FirFilterEffectEvent;
use crate::entities::effects::freeverb::FreeverbEffectEvent;
use crate::entities::effects::lpf::LPFEffectEvent;
use crate::entities::effects::multiplexer::MultiplexerEffectEvent;
use crate::entities::effects::soft_clip::SoftClipEffectEvent;
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
    Bus(BusEvent),
    #[cfg(test)]
    TestSource(crate::entities::sources::TestSourceEvent),
    #[cfg(test)]
    TestEffect(crate::entities::effects::TestEffectEvent),

    // Sources events
    MuxSource(MultiplexerSourceEvent),
    Waveform(WaveformSourceEvent),
    Actors(ActorsSourceEvent),

    // Effects events
    MuxEffect(MultiplexerEffectEvent),
    FirFilter(FirFilterEffectEvent),
    Freeverb(FreeverbEffectEvent),
    LPF(LPFEffectEvent),
    SoftClip(SoftClipEffectEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EventTargetId(usize);

impl std::fmt::Display for EventTargetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventTargetId({})", self.0)
    }
}

impl EventTargetId {
    pub(crate) fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // Zero is reserved for the default target
        EventTargetId(id)
    }

    pub(crate) fn as_usize(&self) -> usize {
        self.0
    }
}

pub(crate) type EventDispatcher = fn(*mut u8, &Event);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventTarget {
    dispatcher: EventDispatcher,
    id: EventTargetId,
    ptr: *mut u8,
}

unsafe impl Send for EventTarget {}
unsafe impl Sync for EventTarget {}

impl Default for EventTarget {
    fn default() -> Self {
        EventTarget {
            dispatcher: |_, _| {},
            id: EventTargetId::new(),
            ptr: std::ptr::null_mut(),
        }
    }
}

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
