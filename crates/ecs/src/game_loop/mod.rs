use crate::events::{ExitEvent, InterSyncEvent, TickEvent};
use crate::game_loop::monitor::{DummyGameLoopMonitor, GameLoopMonitor, GameLoopMonitorTrait};
use crate::game_loop::sync::{
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

/// Event sent every second with monitoring data about the game loop.
#[derive(GlobalEvent, Debug, Clone)]
pub struct GameLoopMonitorEvent {
    pub cycle_time: MonitorSample<Duration>,
    pub tps: MonitorSample<f32>,
    pub load: MonitorSample<f32>,
}

pub struct GameLoopProxy {
    stop_signal: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for GameLoopProxy {
    fn drop(&mut self) {
        info!("GameLoopProxy dropped, stopping game loop");
        self.stop_signal
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            if let Err(e) = handle.join() {
                error!("Joining game loop thread failed: {:?}", e);
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum GameLoopError {
    #[error("Failed to initialize the game loop: {0}")]
    WorldInitError(#[from] anyhow::Error),
}

pub trait InitWorld = FnOnce(&mut World) -> anyhow::Result<()> + Send + Sync + 'static;

impl GameLoopProxy {
    /// Runs the game loop of the application.
    /// Every `tps` ticks per second, it sends a `Tick` event to the ECS.
    /// You can stop the loop by sending a `ExitEvent` event to the ECS.
    ///
    /// The loop will synchronize with the given `Rendezvous` object,
    /// allowing it to run in a multi-threaded environment.
    pub fn new_synchronized(
        before_frame: Rendezvous,
        after_frame: Rendezvous,
        init: impl InitWorld,
    ) -> Result<GameLoopProxy, GameLoopError> {
        GameLoopProxy::new_inner(
            RendezvousSynchronization(before_frame),
            RendezvousSynchronization(after_frame),
            DummyGameLoopMonitor,
            init,
        )
    }

    /// Same as `synchronized_loop`, but it will also send monitoring data every second
    /// to the ECS as `MainLoopMonitorEvent` events.
    /// That may affect the performance of the main loop.
    pub fn new_synchronized_with_monitoring(
        before_frame: Rendezvous,
        after_frame: Rendezvous,
        init: impl InitWorld,
    ) -> Result<GameLoopProxy, GameLoopError> {
        GameLoopProxy::new_inner(
            RendezvousSynchronization(before_frame),
            RendezvousSynchronization(after_frame),
            GameLoopMonitor::new(),
            init,
        )
    }

    /// Same as `synchronized_loop`, but it will run without any synchronization.
    /// You can specify the target tick rate in ticks per second.
    pub fn new_unsynchronized(
        tick_rate: f32,
        init: impl InitWorld,
    ) -> Result<GameLoopProxy, GameLoopError> {
        GameLoopProxy::new_inner(
            DummySynchronization,
            FixedRateSynchronization::new(tick_rate),
            DummyGameLoopMonitor,
            init,
        )
    }

    /// Same as `unsynchronized_loop`, but it will also send monitoring data every second
    /// to the ECS as `MainLoopMonitorEvent` events.
    pub fn new_unsynchronized_with_monitoring(
        tick_rate: f32,
        init: impl InitWorld,
    ) -> Result<GameLoopProxy, GameLoopError> {
        GameLoopProxy::new_inner(
            DummySynchronization,
            FixedRateSynchronization::new(tick_rate),
            GameLoopMonitor::new(),
            init,
        )
    }

    fn new_inner<M>(
        before_frame: impl Synchronization,
        after_frame: impl Synchronization,
        mut monitor: M,
        init: impl InitWorld,
    ) -> Result<GameLoopProxy, GameLoopError>
    where
        M: GameLoopMonitorTrait + 'static,
    {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();
        let handle = Some(
            Builder::new()
                .name("GameLoop".to_string())
                .spawn(move || {
                    info!("Game loop thread started");

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

                    info!("Game loop thread stopped");
                })
                .unwrap(),
        );

        Ok(GameLoopProxy {
            stop_signal: stop_signal_clone,
            handle,
        })
    }
}
