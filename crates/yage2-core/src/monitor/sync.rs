use crate::monitor::MonitorSample;
use std::time::{Duration, Instant};

/// Allows measuring time of some operation
pub struct Stopwatch {
    wma_factor: f32,
    sample: MonitorSample<Duration>,
    start: Instant,
}

pub struct StopwatchGuard<'a> {
    stopwatch: &'a mut Stopwatch,
}

impl Stopwatch {
    /// Creates a new stopwatch.
    /// `wma_factor` is a weight factor for the weighted moving average.
    /// It should be in the range (0.0, 1.0]. 1.0 means that the average will be
    /// equal to the last sample, values closer to 0.0 mean that the average will be
    /// more stable and less sensitive to the last sample.
    pub fn new(wma_factor: f32) -> Self {
        Self {
            wma_factor: wma_factor.clamp(0.01, 1.0),
            sample: MonitorSample::new(
                Duration::from_millis(u64::MAX),
                Duration::from_millis(0),
                Duration::from_millis(0),
            ),
            start: Instant::now(),
        }
    }

    /// Starts the stopwatch
    #[inline(always)]
    pub fn start(&mut self) {
        self.start = Instant::now();
    }

    /// Stops the stopwatch and returns the elapsed time
    #[inline(always)]
    pub fn stop(&mut self) {
        let elapsed = self.start.elapsed();

        // Update the sample
        let old = self.sample.average.as_millis() as f32;
        let new = elapsed.as_millis() as f32;
        let average = old + (new - old) * self.wma_factor;

        self.sample = MonitorSample::new(
            self.sample.min().min(elapsed),
            self.sample.max().max(elapsed),
            Duration::from_millis(average as u64),
        );
    }

    #[inline(always)]
    pub fn get(&self) -> MonitorSample<Duration> {
        self.sample
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.sample = MonitorSample::new(
            self.sample.average(),
            self.sample.average(),
            self.sample.average(),
        );
    }

    /// Provides a mechanism to track elapsed time within a specific scope.
    /// When the returned `StopwatchGuard` is dropped (goes out of scope), it automatically stops
    /// the stopwatch or performs any necessary cleanup related to the timing.
    #[inline(always)]
    pub fn scoped(&mut self) -> StopwatchGuard {
        StopwatchGuard { stopwatch: self }
    }
}

impl<'a> StopwatchGuard<'a> {
    fn new(stopwatch: &'a mut Stopwatch) -> Self {
        stopwatch.start();
        StopwatchGuard { stopwatch }
    }
}

impl Drop for StopwatchGuard<'_> {
    fn drop(&mut self) {
        self.stopwatch.stop();
    }
}

/// Allows counting the number of operations performed in a some
/// specific time (usually one second). The more consistent the update
/// frequency, the more accurate the result will be.
pub struct Counter {
    period: Duration,
    last_update: Instant,
    wma_factor: f32,
    sample: MonitorSample<f32>,
    counter: usize,
}

impl Counter {
    /// Creates a new instance of the struct with the specified parameters.
    /// `period` specifies the time interval you must call `update` method to update
    /// the sample. The closer the actual update time is to the `period`,
    /// the more accurate result you'll get.
    ///
    /// `wma_factor` is a weight factor for the weighted moving average.
    /// It should be in the range (0.0, 1.0]. 1.0 means that the average will be
    /// equal to the last sample, values closer to 0.0 mean that the average will be
    /// more stable and less sensitive to the last sample.
    pub fn new(period: Duration, wma_factor: f32) -> Self {
        Self {
            period,
            last_update: Instant::now(),
            wma_factor,
            sample: MonitorSample::new(f32::MAX, 0.0, 0.0),
            counter: 0,
        }
    }

    #[inline(always)]
    pub fn count(&mut self, count: usize) {
        self.counter += count;
    }

    #[inline(always)]
    pub fn update(&mut self) {
        let elapsed = self.last_update.elapsed();
        let counter = self.counter as f32;
        let counter = if counter == 0.0 {
            0.0
        } else {
            let required_period = self.period.as_micros() as f32;
            (counter * elapsed.as_micros() as f32) / required_period
        };

        self.sample = MonitorSample::new(
            self.sample.min().min(counter),
            self.sample.max().max(counter),
            self.sample.average() + (counter - self.sample.average()) * self.wma_factor,
        );

        self.last_update = Instant::now();
        self.counter = 0;
    }

    #[inline(always)]
    pub fn get(&self) -> MonitorSample<f32> {
        self.sample
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.sample = MonitorSample::new(
            self.sample.average(),
            self.sample.average(),
            self.sample.average(),
        );
        self.counter = 0;
    }
}
