use crate::world::monitor::{DummyWorldLoopMonitor, WorldLoopMonitor, WorldLoopMonitorTrait};
use crate::world::sync::{
    DummySynchronization, FixedRateSynchronization, RendezvousSynchronization, Synchronization,
};
use crate::world::{WorldLoop, WorldLoopError, WorldLoopTickResult};
use dawn_util::rendezvous::Rendezvous;
use log::{error, info};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};

/// World loop Proxy is a wrapper around the WorldLoop that allows to spawn
/// a new World Loop thread.
/// For simpler single-threaded applications consider using `WorldLoop` directly.
/// The WorldLoopProxy will stop the World Loop thread when dropped.
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

impl WorldLoopProxy {
    /// Spawns a new World Loop thread with external synchronization.
    /// The `before_frame` and `after_frame` rendezvous points are used to synchronize
    /// the World loop with other threads in two points.
    pub fn new_synchronized(
        before_frame: Rendezvous,
        after_frame: Rendezvous,
        init: impl crate::world::InitWorld,
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
        init: impl crate::world::InitWorld,
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
        init: impl crate::world::InitWorld,
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
        init: impl crate::world::InitWorld,
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
        init: impl crate::world::InitWorld,
    ) -> Result<WorldLoopProxy, WorldLoopError>
    where
        M: WorldLoopMonitorTrait + 'static,
    {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_signal_clone = stop_signal.clone();
        let handle = Some(
            Builder::new()
                .name("world".to_string())
                .spawn(move || {
                    info!("World loop thread started");
                    let mut world_loop = WorldLoop::new_inner(
                        before_frame.clone(),
                        after_frame.clone(),
                        monitor,
                        init,
                    )
                    .unwrap();

                    while !stop_signal.load(std::sync::atomic::Ordering::SeqCst) {
                        match world_loop.tick() {
                            WorldLoopTickResult::Continue => {}
                            WorldLoopTickResult::Exit => break,
                        }
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
