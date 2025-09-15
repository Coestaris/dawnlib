use dawn_util::rendezvous::Rendezvous;
use log::warn;
use std::panic::UnwindSafe;
use web_time::Duration;

pub trait Synchronization: Send + Sync + 'static + Clone + UnwindSafe {
    fn wait(&self, _elapsed: Duration) {}
    fn unlock(&self) {}
}

#[derive(Clone)]
pub struct RendezvousSynchronization(pub Rendezvous);

impl Synchronization for RendezvousSynchronization {
    fn wait(&self, _: Duration) {
        self.0.wait();
    }

    fn unlock(&self) {
        self.0.unlock();
    }
}

#[derive(Clone)]
pub struct FixedRateSynchronization {
    target_duration: Duration,
}

impl FixedRateSynchronization {
    pub fn new(tick_rate: f32) -> Self {
        let target_duration = Duration::from_secs_f32(1.0 / tick_rate);
        Self { target_duration }
    }
}

impl Synchronization for FixedRateSynchronization {
    fn wait(&self, elapsed: Duration) {
        if elapsed < self.target_duration {
            let sleep_duration = self.target_duration - elapsed;
            std::thread::sleep(sleep_duration);
        } else {
            warn!(
                "Tick took longer than expected: {:.3} seconds",
                elapsed.as_secs_f32()
            );
        }
    }

    fn unlock(&self) {}
}

#[derive(Clone)]
pub struct DummySynchronization;

impl Synchronization for DummySynchronization {
    fn wait(&self, _: Duration) {}
    fn unlock(&self) {}
}
