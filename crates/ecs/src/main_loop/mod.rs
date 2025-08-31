use crate::events::{ExitEvent, InterSyncEvent, TickEvent};
use crate::main_loop::monitor::{DummyMainLoopMonitor, MainLoopMonitor, MainLoopMonitorTrait};
use crate::main_loop::sync::{
    DummySynchronization, FixedRateSynchronization, RendezvousSynchronization, Synchronization,
};
use dawn_util::profile::MonitorSample;
use dawn_util::rendezvous::Rendezvous;
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver};
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::prelude::World;
use log::info;
use std::time::{Duration, Instant};

mod monitor;
mod sync;

/// Event sent every second with monitoring data about the main loop.
#[derive(GlobalEvent, Debug, Clone)]
pub struct MainLoopMonitorEvent {
    pub cycle_time: MonitorSample<Duration>,
    pub tps: MonitorSample<f32>,
    pub load: MonitorSample<f32>,
}

/// Runs the main loop of the application.
/// Every `tps` ticks per second, it sends a `Tick` event to the ECS.
/// You can stop the loop by sending a `ExitEvent` event to the ECS.
///
/// The loop will synchronize with the given `Rendezvous` object,
/// allowing it to run in a multi-threaded environment.
pub fn synchronized_loop(world: &mut World, before_frame: Rendezvous, after_frame: Rendezvous) {
    run_loop_inner(
        world,
        RendezvousSynchronization(before_frame),
        RendezvousSynchronization(after_frame),
        DummyMainLoopMonitor,
    );
}

/// Same as `synchronized_loop`, but it will also send monitoring data every second
/// to the ECS as `MainLoopMonitorEvent` events.
/// That may affect the performance of the main loop.
pub fn synchronized_loop_with_monitoring(
    world: &mut World,
    before_frame: Rendezvous,
    after_frame: Rendezvous,
) {
    run_loop_inner(
        world,
        RendezvousSynchronization(before_frame),
        RendezvousSynchronization(after_frame),
        MainLoopMonitor::new(),
    );
}

/// Same as `synchronized_loop`, but it will run without any synchronization.
/// You can specify the target tick rate in ticks per second.
pub fn unsynchronized_loop(world: &mut World, tick_rate: f32) {
    run_loop_inner(
        world,
        DummySynchronization,
        FixedRateSynchronization::new(tick_rate),
        DummyMainLoopMonitor,
    );
}

/// Same as `unsynchronized_loop`, but it will also send monitoring data every second
/// to the ECS as `MainLoopMonitorEvent` events.
pub fn unsynchronized_loop_with_monitoring(world: &mut World, tick_rate: f32) {
    run_loop_inner(
        world,
        DummySynchronization,
        FixedRateSynchronization::new(tick_rate),
        MainLoopMonitor::new(),
    );
}

fn run_loop_inner<M>(
    world: &mut World,
    before_frame: impl Synchronization,
    after_frame: impl Synchronization,
    mut monitor: M,
) where
    M: MainLoopMonitorTrait + 'static,
{
    #[derive(Component, Debug)]
    struct PrivateData {
        stopped: bool,
    }

    fn stop_event_loop_handler(_: Receiver<ExitEvent>, mut d: Single<&mut PrivateData>) {
        d.stopped = true;
    }

    // Insert a private data component to track the stopped state
    let entity = world.spawn();
    world.insert(entity, PrivateData { stopped: false });
    world.add_handler(stop_event_loop_handler.low());

    let mut prev_tick = Instant::now();
    let loop_start = Instant::now();
    let mut frame = 0;

    loop {
        monitor.cycle(world);
        before_frame.wait(Duration::from_secs(0));

        // Check if the event loop should stop
        if let Some(private_data) = world.get::<PrivateData>(entity) {
            if private_data.stopped {
                after_frame.unlock();
                before_frame.unlock();
                info!("Stopping event loop");
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
}
