use evenio::event::GlobalEvent;

pub trait PassEventTrait = 'static + Clone + Send + Sync + Sized;

/// Targeted event of the render pass.
/// Used to asynchronously send events to the render pass.
#[derive(GlobalEvent, Debug, Clone)]
pub struct RenderPassEvent<E: PassEventTrait> {
    target_id: RenderPassTargetId,
    event: E,
}

impl<E: PassEventTrait> RenderPassEvent<E> {
    pub fn new(target_id: RenderPassTargetId, event: E) -> Self {
        RenderPassEvent { target_id, event }
    }

    #[inline(always)]
    pub(crate) fn get_target_id(&self) -> RenderPassTargetId {
        self.target_id
    }

    #[inline(always)]
    pub(crate) fn get_event(&self) -> &E {
        &self.event
    }
}

/// Unique identifier for a render pass target.
/// This ID is used to identify the target of events dispatched to render passes.
/// It is generated automatically and is unique across all render pass targets.
/// The ID starts from 1, as 0 is reserved for the default target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RenderPassTargetId(usize);

impl std::fmt::Display for RenderPassTargetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RenderPassTargetId({})", self.0)
    }
}

impl RenderPassTargetId {
    /// Creates a new unique `RenderPassTargetId`.
    pub fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // Zero is reserved for the default target
        RenderPassTargetId(id)
    }

    pub(crate) fn as_usize(&self) -> usize {
        self.0
    }
}

type EventDispatcher<E> = fn(*mut u8, &E);

/// Describes a render pass event target.
/// This struct is used by the dispatcher to address
/// events to specific render pass targets.
#[derive(Copy, Clone, Debug, Hash)]
pub struct PassEventTarget<E: PassEventTrait> {
    dispatcher: EventDispatcher<E>,
    id: RenderPassTargetId,
    ptr: *mut u8,
}

unsafe impl<E: PassEventTrait> Send for PassEventTarget<E> {}
unsafe impl<E: PassEventTrait> Sync for PassEventTarget<E> {}

impl<E: PassEventTrait> Default for PassEventTarget<E> {
    fn default() -> Self {
        PassEventTarget {
            dispatcher: |_, _| {},
            id: RenderPassTargetId::new(),
            ptr: std::ptr::null_mut(),
        }
    }
}

impl<E: PassEventTrait> PassEventTarget<E> {
    pub fn new<T>(dispatcher: EventDispatcher<E>, id: RenderPassTargetId, ptr: &T) -> Self {
        PassEventTarget {
            dispatcher,
            id,
            ptr: ptr as *const T as *mut u8,
        }
    }

    pub(crate) fn get_id(&self) -> RenderPassTargetId {
        self.id
    }

    #[inline(always)]
    pub(crate) fn dispatch(&self, event: &E) {
        (self.dispatcher)(self.ptr, event);
    }
}
