use evenio::event::GlobalEvent;

#[derive(GlobalEvent, Debug, Clone)]
pub struct RenderPassEvent<E>
where
    E: 'static,
{
    target_id: RenderPassTargetId,
    event: E,
}

impl<E> RenderPassEvent<E> {
    pub fn new(target_id: RenderPassTargetId, event: E) -> Self {
        RenderPassEvent { target_id, event }
    }

    #[inline(always)]
    pub fn get_target_id(&self) -> RenderPassTargetId {
        self.target_id
    }

    #[inline(always)]
    pub fn get_event(&self) -> &E {
        &self.event
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RenderPassTargetId(usize);

impl std::fmt::Display for RenderPassTargetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RenderPassTargetId({})", self.0)
    }
}

impl RenderPassTargetId {
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

pub(crate) type EventDispatcher<E> = fn(*mut u8, &E);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PassEventTarget<E>
{
    dispatcher: EventDispatcher<E>,
    id: RenderPassTargetId,
    ptr: *mut u8,
}

unsafe impl<E> Send for PassEventTarget<E> {}
unsafe impl<E> Sync for PassEventTarget<E> {}

impl<E> Default for PassEventTarget<E> {
    fn default() -> Self {
        PassEventTarget {
            dispatcher: |_, _| {},
            id: RenderPassTargetId::new(),
            ptr: std::ptr::null_mut(),
        }
    }
}

impl<E> PassEventTarget<E> {
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
