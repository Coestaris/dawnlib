use crate::time::current_us;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::Arc;

pub struct ProfileFrame {
    min: f32,
    average: f32,
    max: f32,
}

impl ProfileFrame {
    pub fn new(min: f32, average: f32, max: f32) -> Self {
        Self { min, average, max }
    }

    pub fn min(&self) -> f32 {
        self.min
    }

    pub fn average(&self) -> f32 {
        self.average
    }

    pub fn max(&self) -> f32 {
        self.max
    }
}

pub struct TickProfiler {
    start: Arc<AtomicU64>,
    ticks: Arc<AtomicU32>,
    start_ticks: Arc<AtomicU32>,
    min_average_us: Arc<AtomicU64>,
    average_us: Arc<AtomicU64>,
    max_average_us: Arc<AtomicU64>,
    smooth_factor: f32,
}

impl Default for TickProfiler {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl TickProfiler {
    pub fn new(smooth_factor: f32) -> Self {
        Self {
            start: Arc::new(AtomicU64::new(current_us())),
            ticks: Arc::new(AtomicU32::new(0)),
            start_ticks: Arc::new(Default::default()),
            min_average_us: Arc::new(AtomicU64::new(u64::MAX)),
            average_us: Arc::new(Default::default()),
            max_average_us: Arc::new(Default::default()),
            smooth_factor,
        }
    }

    pub fn tick(&self, amount: u32) {
        self.ticks.fetch_add(amount, Relaxed);
    }

    pub fn reset(&self) {
        // self.start.store(current_us(), Relaxed);
        // self.ticks.store(0, Relaxed);
        self.min_average_us
            .store(self.average_us.load(Relaxed), Relaxed);
        self.max_average_us
            .store(self.average_us.load(Relaxed), Relaxed);
    }

    pub fn update(&self) {
        let start_us = self.start.load(Relaxed);
        let end_us = current_us();
        let ticks = self.ticks.load(Relaxed);
        let start_ticks = self.start_ticks.load(Relaxed);

        let elapsed_ticks = ticks.saturating_sub(start_ticks);

        /* Update the average time per tick */
        let average = (1_000_000_000 * elapsed_ticks as u64) / end_us.saturating_sub(start_us);

        /* Smooth the average */
        let mut min_average = self.min_average_us.load(Relaxed);
        let mut average_us = self.average_us.load(Relaxed);
        let mut max_average = self.max_average_us.load(Relaxed);

        average_us = (average as f32 * self.smooth_factor
            + average_us as f32 * (1.0 - self.smooth_factor)) as u64;

        if average < min_average {
            min_average = average;
        }
        if average > max_average {
            max_average = average;
        }

        self.min_average_us.store(min_average, Relaxed);
        self.average_us.store(average_us, Relaxed);
        self.max_average_us.store(max_average, Relaxed);
        self.start_ticks.store(ticks, Relaxed);
        self.start.store(end_us, Relaxed);
    }

    /* In milliseconds */
    pub fn get_frame(&self) -> ProfileFrame {
        let (min_average, average, max_average) = (
            self.min_average_us.load(Relaxed),
            self.average_us.load(Relaxed),
            self.max_average_us.load(Relaxed),
        );

        ProfileFrame::new(
            min_average as f32 / 1000.0,
            average as f32 / 1000.0,
            max_average as f32 / 1000.0,
        )
    }
}

pub struct PeriodProfiler {
    start_us: Arc<AtomicU64>,
    min_us: Arc<AtomicU64>,
    current_us: Arc<AtomicU64>,
    max_us: Arc<AtomicU64>,
    smooth_factor: f32,
}

impl Default for PeriodProfiler {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl PeriodProfiler {
    pub fn new(smooth_factor: f32) -> Self {
        Self {
            start_us: Arc::new(AtomicU64::new(0)),
            min_us: Arc::new(AtomicU64::new(u64::MAX)),
            current_us: Arc::new(AtomicU64::new(0)),
            max_us: Arc::new(AtomicU64::new(0)),
            smooth_factor,
        }
    }

    pub fn start(&self) {
        self.start_us.store(current_us(), Relaxed);
    }

    pub fn end(&self) {
        let start_us = self.start_us.load(Relaxed);
        let end_us = current_us();
        let elapsed_us = end_us.saturating_sub(start_us);

        let (mut min, mut current, mut max) = (
            self.min_us.load(Relaxed),
            self.current_us.load(Relaxed),
            self.max_us.load(Relaxed),
        );

        // Update the min, average, and max averages
        current = (elapsed_us as f32 * self.smooth_factor
            + current as f32 * (1.0 - self.smooth_factor)) as u64;

        if elapsed_us < min {
            min = elapsed_us;
        }
        if elapsed_us > max {
            max = elapsed_us;
        }

        self.min_us.store(min, Relaxed);
        self.current_us.store(current, Relaxed);
        self.max_us.store(max, Relaxed);
    }

    /* In milliseconds */
    pub fn get_frame(&self) -> ProfileFrame {
        // Return the statistics for the period counter
        let min = self.min_us.load(Relaxed);
        let current = self.current_us.load(Relaxed);
        let max = self.max_us.load(Relaxed);

        ProfileFrame::new(
            min as f32 / 1000.0,
            current as f32 / 1000.0,
            max as f32 / 1000.0,
        )
    }
}

pub struct MinMaxProfiler {
    min: Arc<AtomicU64>,
    average: Arc<AtomicU64>,
    max: Arc<AtomicU64>,
}

impl Default for MinMaxProfiler {
    fn default() -> Self {
        Self::new()
    }
}

impl MinMaxProfiler {
    pub fn new() -> Self {
        Self {
            min: Arc::new(AtomicU64::new(u64::MAX)),
            average: Arc::new(AtomicU64::new(0)),
            max: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn reset(&self) {
        self.min.store(u64::MAX, Relaxed);
        self.average.store(0, Relaxed);
        self.max.store(0, Relaxed);
    }

    pub fn update(&self, value: u64) {
        let mut min = self.min.load(Relaxed);
        let mut average = self.average.load(Relaxed);
        let mut max = self.max.load(Relaxed);

        if value < min {
            min = value;
        }
        if value > max {
            max = value;
        }

        average = (average + value) / 2;

        self.min.store(min, Relaxed);
        self.average.store(average, Relaxed);
        self.max.store(max, Relaxed);
    }

    /* In milliseconds */
    pub fn get_stat(&self) -> ProfileFrame {
        ProfileFrame::new(
            self.min.load(Relaxed) as f32,
            self.average.load(Relaxed) as f32,
            self.max.load(Relaxed) as f32,
        )
    }
}
