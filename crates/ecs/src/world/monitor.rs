use crate::world::WorldLoopMonitorEvent;
use dawn_util::profile::{Counter, MonitorSample, Stopwatch};
use evenio::world::World;
use std::time::Instant;

pub(crate) trait WorldLoopMonitorTrait: Send + Sync + 'static {
    fn cycle_start(&mut self) {}
    fn tick_end(&mut self) {}
    fn cycle(&mut self, _world: &mut World) {}
}

pub(crate) struct WorldLoopMonitor {
    cycle_time: Stopwatch,
    tps: Counter,
    las_update: Instant,
    counter: usize,
}

impl WorldLoopMonitor {
    pub fn new() -> Self {
        WorldLoopMonitor {
            cycle_time: Stopwatch::new(0.5),
            tps: Counter::new(0.5),
            las_update: Instant::now(),
            counter: 0,
        }
    }
}

impl WorldLoopMonitorTrait for WorldLoopMonitor {
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

            // Calculate the average load of the game loop
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
            world.send(WorldLoopMonitorEvent {
                cycle_time,
                tps,
                load,
            });
        }
    }
}

pub(crate) struct DummyWorldLoopMonitor;

impl WorldLoopMonitorTrait for DummyWorldLoopMonitor {}
