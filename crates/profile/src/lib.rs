#![feature(trait_alias)]

pub mod sync;

pub trait MonitorSampleTrait = Clone + Sync + Sync;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorSample<D: MonitorSampleTrait> {
    min: D,
    average: D,
    max: D,
}

impl<D: MonitorSampleTrait> MonitorSample<D> {
    pub fn new(min: D, average: D, max: D) -> Self {
        Self { min, average, max }
    }

    #[inline]
    pub fn min(&self) -> D {
        self.min.clone()
    }

    #[inline]
    pub fn average(&self) -> D {
        self.average.clone()
    }

    #[inline]
    pub fn max(&self) -> D {
        self.max.clone()
    }
}
