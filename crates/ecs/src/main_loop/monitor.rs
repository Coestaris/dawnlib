use crate::main_loop::MainLoopMonitorEvent;
use dawn_util::profile::{Counter, MonitorSample, Stopwatch};
use evenio::world::World;
use std::time::{Duration, Instant};

pub(crate) trait MainLoopMonitorTrait {
    fn cycle_start(&mut self) {}
    fn tick_end(&mut self) {}
    fn cycle(&mut self, _world: &mut World) {}
}

pub(crate) struct MainLoopMonitor {
    cycle_time: Stopwatch,
    tps: Counter,
    las_update: Instant,
    counter: usize,
}

impl MainLoopMonitor {
    pub fn new() -> Self {
        MainLoopMonitor {
            cycle_time: Stopwatch::new(0.5),
            tps: Counter::new(0.5),
            las_update: Instant::now(),
            counter: 0,
        }
    }
}

impl MainLoopMonitorTrait for MainLoopMonitor {
    #[inline(always)]
    fn cycle_start(&mut self) {
        self.cycle_time.start();
    }

    #[inline(always)]
    fn tick_end(&mut self) {
        self.cycle_time.stop();
    }

    #[inline(always)]
    fn cycle(&mut self, world: &mut World) {
        self.tps.count(1);

        // Check if one second has passed since the last monitor
        if self.las_update.elapsed().as_millis() >= 200 {
            self.las_update = Instant::now();
            self.tps.update();

            // Calculate the average load of the main loop
            let cycle_time = self.cycle_time.get().unwrap_or_default();
            let tps = self.tps.get().unwrap_or_default();

            let load = MonitorSample::new(
                cycle_time.min().as_secs_f32() * tps.min(),
                cycle_time.average().as_secs_f32() * tps.average(),
                cycle_time.max().as_secs_f32() * tps.max(),
            );

            // Reset the counters each 5 seconds to get more smooth data
            if self.counter % 50 == 0 {
                self.cycle_time.reset();
                self.tps.reset();
            }
            self.counter += 1;

            // Send the data to the ECS
            world.send(MainLoopMonitorEvent {
                cycle_time,
                tps,
                load,
            });
        }
    }
}

pub(crate) struct DummyMainLoopMonitor;

impl MainLoopMonitorTrait for DummyMainLoopMonitor {}
