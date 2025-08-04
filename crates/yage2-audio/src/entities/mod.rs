use std::cell::UnsafeCell;
use crate::entities::events::{AudioEventType, AudioEventTarget, AudioEventTargetId};
use crate::sample::PlanarBlock;
use crate::{SampleRate, SamplesCount};

pub mod bus;
pub mod effects;
pub mod events;
pub mod sinks;
pub mod sources;

#[repr(C)]
#[derive(Debug)]
pub struct NodeCell<T> {
    // Using UnsafeCell to allow interior mutability
    node: UnsafeCell<Box<T>>,
}
unsafe impl<T> Send for NodeCell<T> where T: Send {}
unsafe impl<T> Sync for NodeCell<T> where T: Sync {}

impl<T> NodeCell<T> {
    pub fn new(node: T) -> Self {
        NodeCell {
            node: UnsafeCell::new(Box::new(node)),
        }
    }

    pub(crate) fn as_ref(&self) -> &T {
        unsafe { &*self.node.get() }
    }

    pub(crate) fn as_mut(&self) -> &mut T {
        unsafe { &mut *self.node.get() }
    }
}

pub trait Effect {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        // Default implementation returns an empty vector
        vec![]
    }
    fn dispatch(&mut self, _event: &AudioEventType) {
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
    fn get_targets(&self) -> Vec<AudioEventTarget>;
    fn dispatch(&mut self, _event: &AudioEventType) {
        // Default implementation does nothing
    }

    fn frame_start(&mut self) {
        // Default implementation does nothing
    }

    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32>;
}
