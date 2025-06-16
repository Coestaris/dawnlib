use std::sync::{Arc, Condvar, Mutex};

/// Sync point that allows threads to synchronize at specific points.
/// Allows threads to wait for each other before proceeding.
pub struct Rendezvous {
    state: Arc<RendezvousState>,
}

struct RendezvousState {
    mutex: Mutex<Inner>,
    condvar: Condvar,
    max_val: u32,
}

struct Inner {
    count: u32,
    broken: bool,
}

impl Rendezvous {
    pub fn new(max_val: u32) -> Self {
        Rendezvous {
            state: Arc::new(RendezvousState {
                mutex: Mutex::new(Inner {
                    count: 0,
                    broken: false,
                }),
                condvar: Condvar::new(),
                max_val,
            }),
        }
    }

    /// Waits until `max_val` threads have called `wait()` or `unlock()` is called.
    /// Returns `true` if rendezvous succeeded, `false` if it was broken.
    pub fn wait(&self) -> bool {
        let mut inner = self.state.mutex.lock().unwrap();

        if inner.broken {
            return false;
        }

        inner.count += 1;

        if inner.count >= self.state.max_val {
            // Rendezvous complete, reset for next round
            inner.count = 0;
            self.state.condvar.notify_all();
            true
        } else {
            while inner.count != 0 && !inner.broken {
                inner = self.state.condvar.wait(inner).unwrap();
            }
            !inner.broken
        }
    }

    /// Allows a third party to break the rendezvous and wake all waiting threads.
    pub fn unlock(&self) {
        let mut inner = self.state.mutex.lock().unwrap();
        inner.broken = true;
        inner.count = 0;
        self.state.condvar.notify_all();
    }
}