use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::Arc;
use std::time::SystemTime;

fn current_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn current_us() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

pub struct TickCounter {
    start: Arc<AtomicU64>,
    ticks: Arc<AtomicU32>,
    start_ticks: Arc<AtomicU32>,
    min_average_us: Arc<AtomicU64>,
    average_us: Arc<AtomicU64>,
    max_average_us: Arc<AtomicU64>,
    smooth_factor: f32,
}

impl Default for TickCounter {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl TickCounter {
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

    pub fn tick(&self) {
        self.ticks.fetch_add(1, SeqCst);
    }

    pub fn reset(&self) {
        // self.start.store(current_us(), SeqCst);
        // self.ticks.store(0, SeqCst);
        self.min_average_us
            .store(self.average_us.load(SeqCst), SeqCst);
        self.max_average_us
            .store(self.average_us.load(SeqCst), SeqCst);
    }

    pub fn update(&self) {
        let start_us = self.start.load(SeqCst);
        let end_us = current_us();
        let ticks = self.ticks.load(SeqCst);
        let start_ticks = self.start_ticks.load(SeqCst);

        let elapsed_ticks = ticks.saturating_sub(start_ticks);

        /* Update the average time per tick */
        let average = if  elapsed_ticks == 0 {
            u64::MAX
        } else {
            end_us.saturating_sub(start_us) / elapsed_ticks as u64
        };

        /* Smooth the average */
        let mut min_average = self.min_average_us.load(SeqCst);
        let mut average_us = self.average_us.load(SeqCst);
        let mut max_average = self.max_average_us.load(SeqCst);

        average_us = (average as f32 * self.smooth_factor
            + average_us as f32 * (1.0 - self.smooth_factor)) as u64;
        
        if average < min_average {
            min_average = average;
        }
        if average > max_average {
            max_average = average;
        }

        self.min_average_us.store(min_average, SeqCst);
        self.average_us.store(average_us, SeqCst);
        self.max_average_us.store(max_average, SeqCst);
        self.start_ticks.store(ticks, SeqCst);
        self.start.store(end_us, SeqCst);
    }

    /* In milliseconds */
    pub fn get_stat(&self) -> (f32, f32, f32) {
        let (min_average, average, max_average) = (
            self.min_average_us.load(SeqCst),
            self.average_us.load(SeqCst),
            self.max_average_us.load(SeqCst),
        );

        (
            min_average as f32 / 1000.0,
            average as f32 / 1000.0,
            max_average as f32 / 1000.0,
        )
    }
}

pub struct PeriodCounter {
    start_us: Arc<AtomicU64>,
    min_us: Arc<AtomicU64>,
    current_us: Arc<AtomicU64>,
    max_us: Arc<AtomicU64>,
}

impl Default for PeriodCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl PeriodCounter {
    pub fn new() -> Self {
        Self {
            start_us: Arc::new(AtomicU64::new(0)),
            min_us: Arc::new(AtomicU64::new(u64::MAX)),
            current_us: Arc::new(AtomicU64::new(0)),
            max_us: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn start(&self) {
        self.start_us.store(current_us(), SeqCst);
    }

    pub fn end(&self) {
        let start_us = self.start_us.load(SeqCst);
        let end_us = current_us();
        let elapsed_us = end_us.saturating_sub(start_us);

        let (mut min, mut current, mut max) = (
            self.min_us.load(SeqCst),
            self.current_us.load(SeqCst),
            self.max_us.load(SeqCst),
        );

        // Update the min, average, and max averages
        current = (elapsed_us + current) / 2;
        if elapsed_us < min {
            min = elapsed_us;
        }
        if elapsed_us > max {
            max = elapsed_us;
        }

        self.min_us.store(min, SeqCst);
        self.current_us.store(current, SeqCst);
        self.max_us.store(max, SeqCst);
    }

    pub fn reset(&self) {
        self.start_us.store(self.current_us.load(SeqCst), SeqCst);
        self.min_us.store(self.current_us.load(SeqCst), SeqCst);
    }

    /* In milliseconds */
    pub fn get_stat(&self) -> (f32, f32, f32) {
        // Return the statistics for the period counter
        let min = self.min_us.load(SeqCst);
        let current = self.current_us.load(SeqCst);
        let max = self.max_us.load(SeqCst);

        (
            min as f32 / 1000.0,
            current as f32 / 1000.0,
            max as f32 / 1000.0,
        )
    }
}
