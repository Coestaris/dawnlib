use dawn_profile::sync::{Counter, Stopwatch};
use dawn_profile::MonitorSample;
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver};
use evenio::fetch::Single;
use evenio::world::World;
use glam::*;
use log::{info, warn};
use std::time::{Duration, Instant};
use evenio::handler::IntoHandler;

/// Event sent every tick in the main loop (usually 60 times per second).
/// Can be used to update game logic, render frames, etc.
#[derive(GlobalEvent)]
pub struct Tick {
    /// The time since the last tick in seconds in milliseconds.
    pub delta: f32,
    /// The total time since the start of the main loop in milliseconds.
    pub time: f32,
}

/// Event sent to stop the main loop.
#[derive(GlobalEvent)]
pub struct StopEventLoop;

/// Event sent every second with monitoring data about the main loop.
#[derive(GlobalEvent)]
pub struct MainLoopMonitoring {
    pub cycle_time: MonitorSample<Duration>,
    pub tps: MonitorSample<f32>,
    pub load: MonitorSample<f32>,
}

/// Generic component for storing the 'main camera' of the game.
/// Can be used to identify the listener position in the audio system,
/// or the camera position in the rendering system.
#[derive(Component, Debug)]
pub struct Head {
    pub direction: Vec3,
    pub position: Vec3,
}

trait MainLoopMonitorTrait {
    fn cycle_start(&mut self) {}
    fn tick_end(&mut self) {}
    fn cycle(&mut self, _world: &mut World) {}
}

struct MainLoopMonitor {
    cycle_time: Stopwatch,
    tps: Counter,
    las_update: Instant,
    counter: usize,
}

impl MainLoopMonitor {
    pub fn new() -> Self {
        MainLoopMonitor {
            cycle_time: Stopwatch::new(0.5),
            tps: Counter::new(Duration::from_secs(1), 0.5),
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
        if self.las_update.elapsed().as_secs_f32() >= 1.0 {
            self.las_update = Instant::now();
            self.tps.update();

            // Calculate the average load of the main loop
            let cycle_time = self.cycle_time.get();
            let tps = self.tps.get();
            let load = MonitorSample::new(
                cycle_time.min().as_secs_f32() / tps.min(),
                cycle_time.average().as_secs_f32() / tps.max(),
                cycle_time.max().as_secs_f32() / tps.min(),
            );

            // Reset the counters each 5 seconds to get more smooth data
            if self.counter % 5 == 0 {
                self.cycle_time.reset();
                self.tps.reset();
            }
            self.counter += 1;

            // Send the data to the ECS
            world.send(MainLoopMonitoring {
                cycle_time,
                tps,
                load,
            });
        }
    }
}

struct DummyMainLoopMonitor;

impl MainLoopMonitorTrait for DummyMainLoopMonitor {}

/// Runs the main loop of the application.
/// Every `tps` ticks per second, it sends a `Tick` event to the ECS.
/// You can stop the loop by sending a `StopEventLoop` event to the ECS.
pub fn run_loop(world: &mut World, tps: f32) {
    run_loop_inner(world, tps, DummyMainLoopMonitor);
}

/// Same as `run_loop`, but it will also send monitoring data every second
/// to the ECS as `MainLoopMonitorSample` events.
/// That may affect the performance of the main loop.
pub fn run_loop_with_monitoring(world: &mut World, tps: f32) {
    run_loop_inner(world, tps, MainLoopMonitor::new());
}

fn run_loop_inner<M>(world: &mut World, tps: f32, mut monitor: M)
where
    M: MainLoopMonitorTrait + 'static,
{
    #[derive(Component, Debug)]
    struct PrivateData {
        stopped: bool,
    }

    fn stop_event_loop_handler(_: Receiver<StopEventLoop>, mut d: Single<&mut PrivateData>) {
        d.stopped = true;
    }

    // Insert a private data component to track the stopped state
    let entity = world.spawn();
    world.insert(entity, PrivateData { stopped: false });
    world.add_handler(stop_event_loop_handler.low());

    let mut prev_tick = std::time::Instant::now();
    let loop_start = std::time::Instant::now();

    loop {
        monitor.cycle(world);

        // Check if the event loop should stop
        if let Some(private_data) = world.get::<PrivateData>(entity) {
            if private_data.stopped {
                info!("Stopping event loop");
                break;
            }
        }

        // Remember the start time to keep the loop running at a fixed rate
        let start = std::time::Instant::now();

        // Calculate the delta time
        let delta = start.duration_since(prev_tick).as_secs_f32();
        let total_time = start.duration_since(loop_start).as_secs_f32();

        // Dispatch the Tick event
        monitor.cycle_start();
        world.send(Tick {
            delta,
            time: total_time,
        });
        monitor.tick_end();

        // Update the previous tick time
        prev_tick = start;

        // Sleep to maintain the target ticks per second
        let target_duration = Duration::from_secs_f32(1.0 / tps);
        let elapsed = start.duration_since(prev_tick);
        if elapsed < target_duration {
            let sleep_duration = target_duration - elapsed;
            std::thread::sleep(sleep_duration);
        } else {
            warn!(
                "Tick took longer than expected: {:.3} seconds",
                elapsed.as_secs_f32()
            );
        }
    }
}
