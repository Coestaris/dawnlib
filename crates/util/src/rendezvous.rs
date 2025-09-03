use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::AtomicBool;

/// Sync point that allows threads to synchronize at specific points.
/// Allows threads to wait for each other before proceeding.
#[derive(Clone)]
pub struct Rendezvous(Arc<RendezvousInner>);

struct RendezvousInner {
    mutex: Mutex<Inner>,
    unlocked: Arc<AtomicBool>,
    condvar: Condvar,
    max_val: u32,
}

struct Inner {
    count: u32,
    broken: bool,
}

impl Rendezvous {
    pub fn new(max_val: u32) -> Self {
        Rendezvous(Arc::new(RendezvousInner {
            mutex: Mutex::new(Inner {
                count: 0,
                broken: false,
            }),
            unlocked: Arc::new(AtomicBool::new(false)),
            condvar: Condvar::new(),
            max_val,
        }))
    }

    /// Waits until `max_val` threads have called `wait()` or `unlock()` is called.
    /// Returns `true` if rendezvous succeeded, `false` if it was broken.
    pub fn wait(&self) -> bool {
        // Special case: if already unlocked, return false immediately
        if self.0.unlocked.load(std::sync::atomic::Ordering::SeqCst) {
            return false;
        }

        let mut inner = self.0.mutex.lock().unwrap();

        if inner.broken {
            return false;
        }

        inner.count += 1;

        if inner.count >= self.0.max_val {
            // Rendezvous complete, reset for next round
            inner.count = 0;
            self.0.condvar.notify_all();
            true
        } else {
            while inner.count != 0 && !inner.broken {
                inner = self.0.condvar.wait(inner).unwrap();
            }
            !inner.broken
        }
    }

    /// Allows a third party to break the rendezvous and wake all waiting threads.
    pub fn unlock(&self) {
        // Mark as unlocked
        self.0.unlocked.store(true, std::sync::atomic::Ordering::SeqCst);

        // Wake all waiting threads
        let mut inner = self.0.mutex.lock().unwrap();
        inner.broken = true;
        inner.count = 0;
        self.0.condvar.notify_all();
    }
}
