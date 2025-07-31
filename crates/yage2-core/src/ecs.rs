use crate::profile::{PeriodProfiler, TickProfiler};
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver};
use evenio::fetch::Single;
use evenio::world::World;
use glam::Vec3;
use log::{info, warn};

#[derive(GlobalEvent)]
pub struct Tick {
    pub delta: f32,
    pub time: f32,
}

#[derive(GlobalEvent)]
pub struct StopEventLoop;

#[derive(Component, Debug)]
pub struct Position(Vec3);

#[derive(Component, Debug)]
pub struct Head {
    pub direction: Vec3,
    pub position: Vec3,
}

#[derive(Component, Debug)]
struct PrivateData {
    stopped: bool,
}

fn stop_event_loop_handler(_: Receiver<StopEventLoop>, mut d: Single<&mut PrivateData>) {
    d.stopped = true;
}

pub struct ProfileFrame {
    pub tick_time_min: f32,
    pub tick_time_max: f32,
    pub tick_time_av: f32,
    pub tick_tps_min: f32,
    pub tick_tps_max: f32,
    pub tick_tps_av: f32,
}

struct Profiler<F>
where
    F: FnMut(&ProfileFrame) + Send + Sync + 'static,
{
    handler: Option<F>,
    tick_profiler: TickProfiler,
    period_profiler: PeriodProfiler,
    last_profile_time: std::time::Instant,
}

impl<F> Profiler<F>
where
    F: FnMut(&ProfileFrame) + Send + Sync + 'static,
{
    pub fn new(tps: f32, handler: Option<F>) -> Self {
        Profiler {
            handler,
            tick_profiler: TickProfiler::new(tps),
            period_profiler: PeriodProfiler::new(1.0),
            last_profile_time: std::time::Instant::now(),
        }
    }

    pub fn tick_start(&self) {
        if let Some(_) = &self.handler {
            self.period_profiler.start();
        }
    }

    pub fn tick_end(&mut self) {
        if let Some(_) = &self.handler {
            self.period_profiler.end();
        }
    }

    pub fn profile(&mut self) {
        if let Some(handler) = &mut self.handler {
            self.tick_profiler.tick(1);

            // Check if one second has passed since the last profile
            if self.last_profile_time.elapsed().as_secs_f32() >= 1.0 {
                self.last_profile_time = std::time::Instant::now();
                self.tick_profiler.update();

                let (tick_time_min, tick_time_av, tick_time_max) = self.period_profiler.get_stat();
                let (tick_tps_min, tick_tps_av, tick_tps_max) = self.tick_profiler.get_stat();

                let profile_frame = ProfileFrame {
                    tick_time_min,
                    tick_time_max,
                    tick_time_av,
                    tick_tps_min,
                    tick_tps_max,
                    tick_tps_av,
                };

                // Call the handler with the profile frame
                handler(&profile_frame);
            }
        }
    }
}

pub fn run_loop<F>(world: &mut World, tps: f32, profiler: Option<F>)
where
    F: FnMut(&ProfileFrame) + Send + Sync + 'static,
{
    world.add_handler(stop_event_loop_handler);

    // Insert a private data component to track the stopped state
    let entity = world.spawn();
    world.insert(entity, PrivateData { stopped: false });

    let mut prev_tick = std::time::Instant::now();
    let loop_start = std::time::Instant::now();
    let mut profiler = Profiler::new(tps, profiler);

    loop {
        profiler.profile();

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
