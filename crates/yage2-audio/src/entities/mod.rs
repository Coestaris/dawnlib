use crate::entities::events::{Event, EventTarget, EventTargetId};
use crate::sample::PlanarBlock;
use crate::{SampleRate, SamplesCount};

pub mod bus;
pub mod effects;
pub mod events;
pub mod sinks;
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

pub trait Effect {
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

pub struct BlockInfo {
    sample_index: SamplesCount,
    sample_rate: SampleRate,
}

#[allow(unused)]
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

    #[inline(always)]
    fn time(&self, i: SamplesCount) -> f32 {
        (self.sample_index as f32 + i as f32) / self.sample_rate as f32
    }
}

pub trait Source {
    fn get_targets(&self) -> Vec<EventTarget>;
    fn dispatch(&mut self, _event: &Event) {
        // Default implementation does nothing
    }

    fn frame_start(&mut self) {
        // Default implementation does nothing
    }

    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32>;
}
