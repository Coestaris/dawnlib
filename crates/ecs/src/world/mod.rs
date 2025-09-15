use crate::events::{ExitEvent, InterSyncEvent, TickEvent};
use crate::world::monitor::{DummyWorldLoopMonitor, WorldLoopMonitor, WorldLoopMonitorTrait};
use crate::world::sync::{DummySynchronization, Synchronization};
use dawn_util::profile::MonitorSample;
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver};
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::prelude::{EntityId, World};
use log::error;
use thiserror::Error;
use web_time::{Duration, Instant};

mod monitor;
mod sync;
pub mod threading;

/// Event sent every second with monitoring data about the world loop.
#[derive(GlobalEvent, Debug, Clone)]
pub struct WorldLoopMonitorEvent {
    pub cycle_time: MonitorSample<Duration>,
    pub tps: MonitorSample<f32>,
    pub load: MonitorSample<f32>,
}

#[derive(Debug, Error)]
pub enum WorldLoopError {
    #[error("Failed to initialize the world loop: {0}")]
    WorldInitError(#[from] anyhow::Error),
}

pub trait InitWorld = FnOnce(&mut World) -> anyhow::Result<()> + Send + Sync + 'static;

struct InnerData {
    world: World,
    prev_tick: Instant,
    loop_start: Instant,
    frame: usize,
    private_entity: EntityId,
}

pub struct WorldLoop {
    inner_data: InnerData,
    tick_inner: Box<dyn FnMut(&mut InnerData) -> WorldLoopTickResult>,
}

#[derive(Component, Debug)]
struct PrivateData {
    stopped: bool,
}

pub enum WorldLoopTickResult {
    Continue,
    Exit,
}

impl WorldLoop {
    pub fn new(init: impl InitWorld) -> Result<Self, WorldLoopError> {
        Self::new_inner(
            DummySynchronization,
            DummySynchronization,
            DummyWorldLoopMonitor,
            init,
        )
    }

    pub fn new_with_monitoring(init: impl InitWorld) -> Result<Self, WorldLoopError> {
        Self::new_inner(
            DummySynchronization,
            DummySynchronization,
            WorldLoopMonitor::new(),
            init,
        )
    }

    fn new_inner<M>(
        before_frame: impl Synchronization,
        after_frame: impl Synchronization,
        mut monitor: M,
        init: impl InitWorld,
    ) -> Result<Self, WorldLoopError>
    where
        M: WorldLoopMonitorTrait + 'static,
    {
        let mut world = World::new();
        init(&mut world).unwrap();

        fn stop_event_loop_handler(_: Receiver<ExitEvent>, mut d: Single<&mut PrivateData>) {
            d.stopped = true;
        }

        // Insert a private data component to track the stopped state
        let entity = world.spawn();
        world.insert(entity, PrivateData { stopped: false });
        world.add_handler(stop_event_loop_handler.low());

        Ok(Self {
            inner_data: InnerData {
                world,
                prev_tick: Instant::now(),
                loop_start: Instant::now(),
                frame: 0,
                private_entity: entity,
            },
            tick_inner: Box::new(move |s| {
                // Check if the event loop should stop
                if let Some(private_data) = s.world.get::<PrivateData>(s.private_entity) {
                    if private_data.stopped {
                        return WorldLoopTickResult::Exit;
                    }
                }
                monitor.cycle(&mut s.world);
                before_frame.wait(Duration::from_secs(0));

                // Remember the start time to keep the loop running at a fixed rate
                let start = Instant::now();

                // Calculate the delta time
                let delta = start.duration_since(s.prev_tick).as_secs_f32();
                let total_time = start.duration_since(s.loop_start).as_secs_f32();

                // Dispatch the Tick event
                monitor.cycle_start();
                s.world.send(TickEvent {
                    frame: s.frame,
                    delta,
                    time: total_time,
                });
                monitor.tick_end();
                s.frame += 1;

                // Update the previous tick time
                s.prev_tick = start;
                after_frame.wait(start.elapsed());

                s.world.send(InterSyncEvent { frame: s.frame });

                WorldLoopTickResult::Continue
            }),
        })
    }

    pub fn tick(&mut self) -> WorldLoopTickResult {
        (self.tick_inner)(&mut self.inner_data)
    }
}
