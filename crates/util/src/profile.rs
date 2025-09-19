use log::debug;
use web_time::{Duration, Instant};

/// Utility struct to measure the time taken by a scope
/// and log it when the struct is dropped.
/// Usage:
/// ```
/// use dawn_util::profile::Measure;
/// {
///     let _measure = Measure::new("Some operation".to_string());
///     // Some operation to measure
/// }
/// ```
/// When the scope ends, the time taken by the operation will be logged.
pub struct Measure(String, Instant);

impl Measure {
    pub fn new(message: String) -> Self {
        Measure(message, Instant::now())
    }
}

impl Drop for Measure {
    fn drop(&mut self) {
        debug!("{} in {:?}", self.0, self.1.elapsed());
    }
}

pub trait MonitorSampleTrait = Clone + Sync + Sync + Default;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
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

/// Allows measuring time of some operation
pub struct Stopwatch {
    wma_factor: f32,
    min: Option<Duration>,
    max: Option<Duration>,
    average: Option<Duration>,
    pub start: Instant,
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
            min: None,
            max: None,
            average: None,
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

        if let Some(min) = self.min {
            self.min = Some(min.min(elapsed));
        } else {
            self.min = Some(elapsed);
        }
        if let Some(max) = self.max {
            self.max = Some(max.max(elapsed));
        } else {
            self.max = Some(elapsed);
        }
        if let Some(average) = self.average {
            let old = average.as_micros() as f32;
            let new = elapsed.as_micros() as f32;
            self.average = Some(Duration::from_micros(
                (old * self.wma_factor + new * (1.0 - self.wma_factor)) as u64,
            ));
        } else {
            self.average = Some(elapsed);
        }
    }

    #[inline(always)]
    pub fn get(&self) -> Option<MonitorSample<Duration>> {
        if let Some(min) = self.min {
            if let Some(max) = self.max {
                if let Some(average) = self.average {
                    return Some(MonitorSample::new(min, average, max));
                }
            }
        }
        None
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        // Copy average to min and max
        if let Some(average) = self.average {
            self.min = Some(average);
            self.max = Some(average);
        } else {
            self.min = None;
            self.max = None;
        }
    }

    /// Provides a mechanism to track elapsed time within a specific scope.
    /// When the returned `StopwatchGuard` is dropped (goes out of scope), it automatically stops
    /// the stopwatch or performs any necessary cleanup related to the timing.
    #[inline(always)]
    pub fn scoped(&mut self) -> StopwatchGuard<'_> {
        StopwatchGuard::new(self)
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
    last_update: Instant,
    wma_factor: f32,
    counter: usize,
    min: Option<f32>,
    max: Option<f32>,
    average: Option<f32>,
}

impl Counter {
    /// Creates a new instance of the struct with the specified parameters.
    ///
    /// `wma_factor` is a weight factor for the weighted moving average.
    /// It should be in the range (0.0, 1.0]. 1.0 means that the average will be
    /// equal to the last sample, values closer to 0.0 mean that the average will be
    /// more stable and less sensitive to the last sample.
    pub fn new(wma_factor: f32) -> Self {
        Self {
            last_update: Instant::now(),
            wma_factor,
            counter: 0,
            min: None,
            max: None,
            average: None,
        }
    }

    #[inline(always)]
    pub fn count(&mut self, count: usize) {
        self.counter += count;
    }

    #[inline(always)]
    pub fn update(&mut self) {
        let elapsed = self.last_update.elapsed();
        let elapsed_s = elapsed.as_secs_f32().max(f32::EPSILON);
        let counter = self.counter as f32;

        let current = counter / elapsed_s;

        if let Some(min) = self.min {
            self.min = Some(min.min(current));
        } else {
            self.min = Some(current);
        }
        if let Some(max) = self.max {
            self.max = Some(max.max(current));
        } else {
            self.max = Some(current);
        }
        if let Some(average) = self.average {
            let old = average;
            let new = current;
            self.average = Some(old * self.wma_factor + new * (1.0 - self.wma_factor));
        } else {
            self.average = Some(current);
        }

        self.last_update = Instant::now();
        self.counter = 0;
    }

    /// Returns the number of counts per second agnostic to the period
    #[inline(always)]
    pub fn get(&self) -> Option<MonitorSample<f32>> {
        if let Some(min) = self.min {
            if let Some(max) = self.max {
                if let Some(average) = self.average {
                    return Some(MonitorSample::new(min, average, max));
                }
            }
        }
        None
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        // Copy average to min and max
        if let Some(average) = self.average {
            self.min = Some(average);
            self.max = Some(average);
        } else {
            self.min = None;
            self.max = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stopwatch_empty() {
        let mut stopwatch = Stopwatch::new(0.0);
        let sample = stopwatch.get();
        assert!(sample.is_none());

        stopwatch.reset();
        let sample = stopwatch.get();
        assert!(sample.is_none());
    }

    #[test]
    fn test_stopwatch_single() {
        let mut stopwatch = Stopwatch::new(0.0);
        stopwatch.start();
        std::thread::sleep(Duration::from_millis(100));
        stopwatch.stop();

        // Min, max and average should be the same
        let sample = stopwatch.get().unwrap();
        assert_eq!(sample.min(), sample.max());
        assert_eq!(sample.average(), sample.max());
        // And must be around 100ms
        assert!(sample.average().as_millis() >= 99);
        assert!(sample.average().as_millis() <= 101);

        stopwatch.reset();

        let sample = stopwatch.get().unwrap();
        assert_eq!(sample.min(), sample.max());
        assert_eq!(sample.average(), sample.max());
        assert!(sample.average().as_millis() >= 99);
        assert!(sample.average().as_millis() <= 101);
    }

    #[test]
    fn test_stopwatch_multiple() {
        let mut stopwatch = Stopwatch::new(0.5);

        stopwatch.start();
        std::thread::sleep(Duration::from_millis(50));
        stopwatch.stop();

        stopwatch.start();
        std::thread::sleep(Duration::from_millis(100));
        stopwatch.stop();

        stopwatch.start();
        std::thread::sleep(Duration::from_millis(150));
        stopwatch.stop();

        let sample = stopwatch.get().unwrap();
        assert_eq!(sample.min().as_millis(), 50);
        assert_eq!(sample.max().as_millis(), 150);
        assert!(sample.average().as_millis() > 100);
        assert!(sample.average().as_millis() < 150);

        stopwatch.reset();

        let sample = stopwatch.get().unwrap();
        assert_eq!(sample.min(), sample.max());
        assert_eq!(sample.average(), sample.max());
        assert!(sample.average().as_millis() > 100);
        assert!(sample.average().as_millis() < 150);
    }

    #[test]
    fn test_stopwatch_short() {
        let mut stopwatch = Stopwatch::new(0.5);

        for i in 0..60 {
            stopwatch.start();
            std::thread::sleep(Duration::from_millis(40 + (i % 10)));
            stopwatch.stop();
        }

        let sample = stopwatch.get().unwrap();
        assert_eq!(sample.min().as_millis(), 40);
        assert_eq!(sample.max().as_millis(), 49);
        assert!(sample.average().as_millis() > 44);
        assert!(sample.average().as_millis() < 49);

        stopwatch.reset();

        let sample = stopwatch.get().unwrap();
        assert_eq!(sample.min(), sample.max());
        assert_eq!(sample.average(), sample.max());
        assert!(sample.average().as_millis() > 44);
        assert!(sample.average().as_millis() < 49);
    }

    #[test]
    fn test_counter_empty() {
        let mut counter = Counter::new(0.5);
        let sample = counter.get();
        assert!(sample.is_none());

        counter.reset();
        let sample = counter.get();
        assert!(sample.is_none());

        counter.update();
        let sample = counter.get().unwrap();
        assert_eq!(sample.min(), sample.max());
        assert_eq!(sample.average(), sample.max());
        assert_eq!(sample.min(), 0.0);
        assert_eq!(sample.max(), 0.0);
        assert_eq!(sample.average(), 0.0);
    }

    #[test]
    fn test_counter_second() {
        let mut counter = Counter::new(0.5);

        let mut last_update = Instant::now();
        for _ in 0..300 {
            counter.count(1);
            std::thread::sleep(Duration::from_millis(10));

            if last_update.elapsed().as_millis() >= 1000 {
                last_update = Instant::now();
                counter.update();
            }
        }

        const TOLERANCE: f32 = 0.1;
        const EXPECTED: f32 = 100.0;

        let sample = counter.get().unwrap();
        println!("Counter: {:?}", sample);
        assert_eq!(sample.min() >= EXPECTED * (1.0 - TOLERANCE), true);
        assert_eq!(sample.min() <= EXPECTED * (1.0 + TOLERANCE), true);
        assert_eq!(sample.max() >= EXPECTED * (1.0 - TOLERANCE), true);
        assert_eq!(sample.max() <= EXPECTED * (1.0 + TOLERANCE), true);
        assert_eq!(sample.average() >= EXPECTED * (1.0 - TOLERANCE), true);
        assert_eq!(sample.average() <= EXPECTED * (1.0 + TOLERANCE), true);

        counter.reset();

        let sample = counter.get().unwrap();
        assert_eq!(sample.min(), sample.max());
        assert_eq!(sample.average(), sample.max());
    }

    #[test]
    fn test_counter_100ms() {
        let mut counter = Counter::new(0.5);

        let mut last_update = Instant::now();
        for _ in 0..300 {
            counter.count(1);
            std::thread::sleep(Duration::from_millis(10));

            if last_update.elapsed().as_millis() >= 100 {
                last_update = Instant::now();
                counter.update();
            }
        }

        const TOLERANCE: f32 = 0.1;
        const EXPECTED: f32 = 100.0;
        let sample = counter.get().unwrap();
        println!("Counter: {:?}", sample);

        assert_eq!(sample.min() >= EXPECTED * (1.0 - TOLERANCE), true);
        assert_eq!(sample.min() <= EXPECTED * (1.0 + TOLERANCE), true);
        assert_eq!(sample.max() >= EXPECTED * (1.0 - TOLERANCE), true);
        assert_eq!(sample.max() <= EXPECTED * (1.0 + TOLERANCE), true);
        assert_eq!(sample.average() >= EXPECTED * (1.0 - TOLERANCE), true);
        assert_eq!(sample.average() <= EXPECTED * (1.0 + TOLERANCE), true);

        counter.reset();

        let sample = counter.get().unwrap();
        assert_eq!(sample.min(), sample.max());
        assert_eq!(sample.average(), sample.max());
    }

    #[test]
    fn test_counter_10ms() {
        let mut counter = Counter::new(0.5);

        let mut last_update = Instant::now();
        for _ in 0..300 {
            counter.count(1);
            std::thread::sleep(Duration::from_millis(10));

            if last_update.elapsed().as_millis() >= 10 {
                last_update = Instant::now();
                counter.update();
            }
        }

        const TOLERANCE: f32 = 0.1;
        const EXPECTED: f32 = 100.0;
        let sample = counter.get().unwrap();
        println!("Counter: {:?}", sample);

        assert_eq!(sample.min() >= EXPECTED * (1.0 - TOLERANCE), true);
        assert_eq!(sample.min() <= EXPECTED * (1.0 + TOLERANCE), true);
        assert_eq!(sample.max() >= EXPECTED * (1.0 - TOLERANCE), true);
        assert_eq!(sample.max() <= EXPECTED * (1.0 + TOLERANCE), true);
        assert_eq!(sample.average() >= EXPECTED * (1.0 - TOLERANCE), true);
        assert_eq!(sample.average() <= EXPECTED * (1.0 + TOLERANCE), true);

        counter.reset();

        let sample = counter.get().unwrap();
        assert_eq!(sample.min(), sample.max());
        assert_eq!(sample.average(), sample.max());
    }
}
