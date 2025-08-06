use crate::resources::resource::ResourceID;
use crossbeam_queue::ArrayQueue;
use log::{debug, error};
use std::mem;
use std::ptr::NonNull;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

struct FreePromise {
    pub free_me: Arc<ArrayQueue<ResourceID>>,
    pub freed: Arc<(Mutex<bool>, Condvar)>,
}

impl FreePromise {
    pub fn reset(&self) {
        // Reset the condition variable and the mutex
        let (lock, cvar) = &*self.freed;
        let mut done = lock.lock().unwrap();
        *done = false; // Reset the done state
        cvar.notify_all(); // Notify all waiting threads
    }

    pub fn wait(&self, timeout: Duration) -> bool {
        let (lock, cvar) = &*self.freed;
        let mut done = lock.lock().unwrap();
        if !*done {
            // Wait for the condition variable to be notified
            done = cvar.wait_timeout(done, timeout).unwrap().0;
        }
        *done
    }
}

pub(crate) struct ResourceRef {
    pub id: ResourceID,
    pub promise: FreePromise,
    pub ptr: NonNull<()>,
}

impl Drop for ResourceRef {
    fn drop(&mut self) {
        debug!("Notifying drop for resource: {}", self.id);

        self.promise.reset();

        // Notify that the resource is dropped
        if let Err(_) = self.promise.free_me.push(self.id.clone()) {
            error!("Failed to notify about resource drop: {}", self.id);
        }

        // Notify the condition variable that the resource is freed
        if !self.promise.wait(Duration::from_secs(1)) {
            panic!("Resource dropped while waiting for a resource to notify");
        }
    }
}

impl ResourceRef {
    pub fn new(id: ResourceID, free_promise: FreePromise, ptr: NonNull<()>) -> Self {
        ResourceRef {
            id,
            promise: free_promise,
            ptr,
        }
    }

    pub unsafe fn deref<'a, T>(self) -> &'a T {
        let ptr = self.ptr.as_ptr();
        unsafe { mem::transmute::<*const (), &T>(ptr) }
    }
}
