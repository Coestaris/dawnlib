use crate::profile::{PeriodProfiler, ProfileFrame, TickProfiler};
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver};
use evenio::fetch::Single;
use evenio::world::World;
use glam::Vec3;
use log::{info, warn};

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

/// Event sent every second with profiling data about the main loop.
#[derive(GlobalEvent)]
pub struct MainLoopProfileFrame {
    pub tick_time: ProfileFrame,
    pub tick_tps: ProfileFrame,
}

/// Generic component for storing the 'main camera' of the game.
/// Can be used to identify the listener position in the audio system,
/// or the camera position in the rendering system.
#[derive(Component, Debug)]
pub struct Head {
    pub direction: Vec3,
    pub position: Vec3,
}

trait MainLoopProfilerTrait {
    fn tick_start(&self) {}
    fn tick_end(&mut self) {}
    fn profile(&mut self, world: &mut World) {}
}

struct MainLoopProfiler {
    tick_profiler: TickProfiler,
    period_profiler: PeriodProfiler,
    last_profile_time: std::time::Instant,
}

impl MainLoopProfiler {
    pub fn new() -> Self {
        MainLoopProfiler {
            tick_profiler: TickProfiler::new(0.5),
            period_profiler: PeriodProfiler::new(0.5),
            last_profile_time: std::time::Instant::now(),
        }
    }
}

impl MainLoopProfilerTrait for MainLoopProfiler {
    fn tick_start(&self) {
        self.period_profiler.start();
    }

    fn tick_end(&mut self) {
        self.period_profiler.end();
    }

    fn profile(&mut self, world: &mut World) {
        self.tick_profiler.tick(1);

        // Check if one second has passed since the last profile
        if self.last_profile_time.elapsed().as_secs_f32() >= 1.0 {
            self.last_profile_time = std::time::Instant::now();
            self.tick_profiler.update();

            // Call the handler with the profile frame
            world.send(MainLoopProfileFrame {
                tick_time: self.period_profiler.get_frame(),
                tick_tps: self.tick_profiler.get_frame(),
            });
        }
    }
}

struct DummyMainLoopProfiler;

impl MainLoopProfilerTrait for DummyMainLoopProfiler {}

/// Runs the main loop of the application.
/// Every `tps` ticks per second, it sends a `Tick` event to the ECS.
/// You can stop the loop by sending a `StopEventLoop` event to the ECS.
/// If `use_profiling` is true, it will also send profiling data every second
/// to the ECS as `MainLoopProfileFrame` events.
pub fn run_loop(world: &mut World, tps: f32, use_profiling: bool) {
    if use_profiling {
        run_loop_inner(world, tps, MainLoopProfiler::new());
    } else {
        run_loop_inner(world, tps, DummyMainLoopProfiler);
    }
}

fn run_loop_inner<P>(world: &mut World, tps: f32, mut profiler: P)
where
    P: MainLoopProfilerTrait + 'static,
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
    world.add_handler(stop_event_loop_handler);

    let mut prev_tick = std::time::Instant::now();
    let loop_start = std::time::Instant::now();

    loop {
        profiler.profile(world);

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
        profiler.tick_start();
        world.send(Tick {
            delta,
            time: total_time,
        });
        profiler.tick_end();

        // Update the previous tick time
        prev_tick = start;

        // Sleep to maintain the target ticks per second
        let target_duration = std::time::Duration::from_secs_f32(1.0 / tps);
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
