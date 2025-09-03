use crate::events::{ExitEvent, InterSyncEvent, TickEvent};
use crate::world::monitor::{DummyWorldLoopMonitor, WorldLoopMonitor, WorldLoopMonitorTrait};
use crate::world::sync::{
    DummySynchronization, FixedRateSynchronization, RendezvousSynchronization, Synchronization,
};
use dawn_util::profile::MonitorSample;
use dawn_util::rendezvous::Rendezvous;
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver};
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::prelude::World;
use log::{error, info, warn};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use std::time::{Duration, Instant};
use thiserror::Error;

mod monitor;
mod sync;

/// Event sent every second with monitoring data about the world loop.
#[derive(GlobalEvent, Debug, Clone)]
pub struct WorldLoopMonitorEvent {
    pub cycle_time: MonitorSample<Duration>,
    pub tps: MonitorSample<f32>,
    pub load: MonitorSample<f32>,
}

/// World loop is a wrapper around the evenio's World that runs in a
/// separate thread, that eventually sends TickEvent to the ECS until Exited.
pub struct WorldLoopProxy {
    stop_signal: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for WorldLoopProxy {
    fn drop(&mut self) {
        info!("WorldLoopProxy dropped, stopping the loop thread");
        self.stop_signal
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            if let Err(e) = handle.join() {
                error!("Joining world loop thread failed: {:?}", e);
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum WorldLoopError {
    #[error("Failed to initialize the world loop: {0}")]
    WorldInitError(#[from] anyhow::Error),
}

pub trait InitWorld = FnOnce(&mut World) -> anyhow::Result<()> + Send + Sync + 'static;

impl WorldLoopProxy {
    /// Spawns a new World Loop thread with external synchronization.
    /// The `before_frame` and `after_frame` rendezvous points are used to synchronize
    /// the World loop with other threads in two points.
    pub fn new_synchronized(
        before_frame: Rendezvous,
        after_frame: Rendezvous,
        init: impl InitWorld,
    ) -> Result<WorldLoopProxy, WorldLoopError> {
        WorldLoopProxy::new_inner(
            RendezvousSynchronization(before_frame),
            RendezvousSynchronization(after_frame),
            DummyWorldLoopMonitor,
            init,
        )
    }

    /// Spawns a new World Loop thread with external synchronization and monitoring.
    /// This is the same as `new_synchronized`, but also eventually sends `WorldLoopMonitorEvent` events.
    pub fn new_synchronized_with_monitoring(
        before_frame: Rendezvous,
        after_frame: Rendezvous,
        init: impl InitWorld,
    ) -> Result<WorldLoopProxy, WorldLoopError> {
        WorldLoopProxy::new_inner(
            RendezvousSynchronization(before_frame),
            RendezvousSynchronization(after_frame),
            WorldLoopMonitor::new(),
            init,
        )
    }

    /// Spawns a new World Loop thread that runs at a fixed tick rate.
    /// The loop will try to run at the specified tick rate, but if the processing
    /// takes longer than the tick duration, it will run as fast as possible.
    /// No synchronization with other threads is performed.
    pub fn new_unsynchronized(
        tick_rate: f32,
        init: impl InitWorld,
    ) -> Result<WorldLoopProxy, WorldLoopError> {
        WorldLoopProxy::new_inner(
            DummySynchronization,
            FixedRateSynchronization::new(tick_rate),
            DummyWorldLoopMonitor,
            init,
        )
    }

    /// Spawns a new World Loop thread that runs at a fixed tick rate with monitoring.
    /// This is the same as `new_unsynchronized`, but also eventually sends `WorldLoopMonitorEvent` events.
    pub fn new_unsynchronized_with_monitoring(
        tick_rate: f32,
        init: impl InitWorld,
    ) -> Result<WorldLoopProxy, WorldLoopError> {
        WorldLoopProxy::new_inner(
            DummySynchronization,
            FixedRateSynchronization::new(tick_rate),
            WorldLoopMonitor::new(),
            init,
        )
    }

    fn new_inner<M>(
        before_frame: impl Synchronization,
        after_frame: impl Synchronization,
        mut monitor: M,
        init: impl InitWorld,
    ) -> Result<WorldLoopProxy, WorldLoopError>
    where
        M: WorldLoopMonitorTrait + 'static,
    {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();
        let handle = Some(
            Builder::new()
                .name("worldloop".to_string())
                .spawn(move || {
                    info!("World loop thread started");

                    let mut world = World::new();
                    init(&mut world).unwrap();

                    #[derive(Component, Debug)]
                    struct PrivateData {
                        stopped: bool,
                    }

                    fn stop_event_loop_handler(
                        _: Receiver<ExitEvent>,
                        mut d: Single<&mut PrivateData>,
                    ) {
                        d.stopped = true;
                    }

                    // Insert a private data component to track the stopped state
                    let entity = world.spawn();
                    world.insert(entity, PrivateData { stopped: false });
                    world.add_handler(stop_event_loop_handler.low());

                    let mut prev_tick = Instant::now();
                    let loop_start = Instant::now();
                    let mut frame = 0;

                    while !stop_signal.load(std::sync::atomic::Ordering::SeqCst) {
                        monitor.cycle(&mut world);
                        before_frame.wait(Duration::from_secs(0));

                        // Check if the event loop should stop
                        if let Some(private_data) = world.get::<PrivateData>(entity) {
                            if private_data.stopped {
                                break;
                            }
                        }

                        // Remember the start time to keep the loop running at a fixed rate
                        let start = Instant::now();

                        // Calculate the delta time
                        let delta = start.duration_since(prev_tick).as_secs_f32();
                        let total_time = start.duration_since(loop_start).as_secs_f32();

                        // Dispatch the Tick event
                        monitor.cycle_start();
                        world.send(TickEvent {
                            frame,
                            delta,
                            time: total_time,
                        });
                        monitor.tick_end();
                        frame += 1;

                        // Update the previous tick time
                        prev_tick = start;
                        after_frame.wait(start.elapsed());

                        world.send(InterSyncEvent { frame });
                    }

                    after_frame.unlock();
                    before_frame.unlock();

                    info!("World loop thread stopped");
                })
                .unwrap(),
        );

        Ok(WorldLoopProxy {
            stop_signal: stop_signal_clone,
            handle,
        })
    }
}
