use crate::entities::bus::BusEvent;
use crate::entities::effects::fir::FirFilterEffectEvent;
use crate::entities::effects::freeverb::FreeverbEffectEvent;
use crate::entities::effects::multiplexer::MultiplexerEffectEvent;
use crate::entities::effects::soft_clip::SoftClipEffectEvent;
use crate::entities::sources::actor::ActorsSourceEvent;
use crate::entities::sources::multiplexer::MultiplexerSourceEvent;
use crate::entities::sources::waveform::WaveformSourceEvent;
use evenio::prelude::GlobalEvent;

#[derive(GlobalEvent, Debug, Clone)]
pub struct AudioEvent {
    target_id: AudioEventTargetId,
    event: AudioEventType,
}

impl AudioEvent {
    pub fn new(target_id: AudioEventTargetId, event: AudioEventType) -> Self {
        AudioEvent { target_id, event }
    }

    #[inline(always)]
    pub fn get_target_id(&self) -> AudioEventTargetId {
        self.target_id
    }

    #[inline(always)]
    pub fn get_event(&self) -> &AudioEventType {
        &self.event
    }
}

#[derive(Debug, Clone)]
pub enum AudioEventType {
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
    SoftClip(SoftClipEffectEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AudioEventTargetId(usize);

impl std::fmt::Display for AudioEventTargetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AudioEventTargetId({})", self.0)
    }
}

impl AudioEventTargetId {
    pub(crate) fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // Zero is reserved for the default target
        AudioEventTargetId(id)
    }

    pub(crate) fn as_usize(&self) -> usize {
        self.0
    }
}

pub(crate) type EventDispatcher = fn(*mut u8, &AudioEventType);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AudioEventTarget {
    dispatcher: EventDispatcher,
    id: AudioEventTargetId,
    ptr: *mut u8,
}

unsafe impl Send for AudioEventTarget {}
unsafe impl Sync for AudioEventTarget {}

impl Default for AudioEventTarget {
    fn default() -> Self {
        AudioEventTarget {
            dispatcher: |_, _| {},
            id: AudioEventTargetId::new(),
            ptr: std::ptr::null_mut(),
        }
    }
}

impl AudioEventTarget {
    pub(crate) fn new<T>(dispatcher: EventDispatcher, id: AudioEventTargetId, ptr: &T) -> Self {
        AudioEventTarget {
            dispatcher,
            id,
            ptr: ptr as *const T as *mut u8,
        }
    }

    pub(crate) fn get_id(&self) -> AudioEventTargetId {
        self.id
    }

    #[inline(always)]
    pub(crate) fn dispatch(&self, event: &AudioEventType) {
        (self.dispatcher)(self.ptr, event);
    }
}
