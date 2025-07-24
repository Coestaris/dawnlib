use log::info;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::{Scope, ScopedJoinHandle};

#[cfg(target_os = "linux")]
#[allow(unused)]
mod native_impl {
    use crate::threads::ThreadPriority;
    use log::{debug, warn};
    use std::fs::File;
    use std::io::Read;

    use libc::{self, pid_t};

    pub type NativeId = pid_t;

    #[derive(Debug, Default)]
    pub struct NativeData {
        prev_proc_time: f32,
    }

    pub fn get_thread_native_id() -> NativeId {
        unsafe { libc::syscall(libc::SYS_gettid) as pid_t }
    }

    fn get_proc_time(native_id: NativeId) -> f32 {
        let path = format!("/proc/self/task/{}/stat", native_id);
        let mut contents = String::new();
        if File::open(&path)
            .and_then(|mut f| f.read_to_string(&mut contents))
            .is_err()
        {
            warn!("Failed to read stat for thread {}", native_id);
            return 0.0;
        }

        let parts: Vec<&str> = contents.split_whitespace().collect();
        if parts.len() < 15 {
            warn!("Unexpected stat format for thread {}", native_id);
            return 0.0;
        }

        let utime: f32 = parts[13].parse().unwrap_or(0.0);
        let stime: f32 = parts[14].parse().unwrap_or(0.0);
        let total_time_ticks = utime + stime;

        let clk_tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f32;
        total_time_ticks / clk_tck // CPU time в секундах
    }

    pub fn get_cpu_utilization(data: &mut NativeData, native_id: NativeId) -> f32 {
        let current_proc_time = get_proc_time(native_id);
        let cpu_utilization = if data.prev_proc_time > 0.0 {
            (current_proc_time - data.prev_proc_time) / 1.0 // Assuming 1 second interval
        } else {
            0.0
        };
        data.prev_proc_time = current_proc_time;
        cpu_utilization
    }

    pub fn set_my_priority(priority: ThreadPriority) {
        let tid = get_thread_native_id();

        let (policy, sched_priority, nice_value) = match priority {
            ThreadPriority::Low => (libc::SCHED_OTHER, 0, 10),
            ThreadPriority::Normal => (libc::SCHED_OTHER, 0, 0),
            ThreadPriority::High => (libc::SCHED_RR, 10, -5),
            ThreadPriority::Realtime => (libc::SCHED_FIFO, 15, -10),
        };

        unsafe {
            if libc::setpriority(libc::PRIO_PROCESS, tid as u32, nice_value) != 0 {
                warn!(
                    "Failed to set nice level for tid {}: {}",
                    tid,
                    std::io::Error::last_os_error()
                );
            }

            let mut param = libc::sched_param { sched_priority };

            let pthread = libc::pthread_self();
            if libc::pthread_setschedparam(pthread, policy, &mut param) != 0 {
                warn!(
                    "Failed to set pthread policy for tid {}: {}",
                    tid,
                    std::io::Error::last_os_error()
                );
            }
        }

        debug!("Set priority for thread {} to {:?}", tid, priority);
    }
}

#[cfg(target_os = "windows")]
#[allow(unused)]
mod native_impl {
    use crate::threads::ThreadPriority;
    use log::debug;

    pub type NativeId = usize;

    #[derive(Debug, Default)]
    pub struct NativeData {}

    pub fn get_thread_native_id() -> NativeId {
        // Placeholder for actual native thread ID retrieval logic
        1
    }

    pub fn get_cpu_utilization(_: &mut NativeData, _: NativeId) -> f32 {
        // Placeholder for actual CPU utilization retrieval logic
        0.0
    }

    pub fn set_my_priority(priority: ThreadPriority) {
        // Placeholder for actual thread priority setting logic
        match priority {
            ThreadPriority::Low => debug!("Setting thread priority to Low"),
            ThreadPriority::Normal => debug!("Setting thread priority to Normal"),
            ThreadPriority::High => debug!("Setting thread priority to High"),
            ThreadPriority::Realtime => debug!("Setting thread priority to Realtime"),
        }
    }
}

#[cfg(target_os = "macos")]
#[allow(unused)]
mod native_impl {
    use crate::threads::ThreadPriority;
    use log::debug;

    pub type NativeId = usize;

    #[derive(Debug, Default)]
    pub struct NativeData {}

    pub fn get_thread_native_id() -> NativeId {
        // Placeholder for actual native thread ID retrieval logic
        1
    }

    pub fn get_cpu_utilization(_: &mut NativeData, _: NativeId) -> f32 {
        // Placeholder for actual CPU utilization retrieval logic
        0.0
    }

    pub fn set_my_priority(priority: ThreadPriority) {
        // Placeholder for actual thread priority setting logic
        match priority {
            ThreadPriority::Low => debug!("Setting thread priority to Low"),
            ThreadPriority::Normal => debug!("Setting thread priority to Normal"),
            ThreadPriority::High => debug!("Setting thread priority to High"),
            ThreadPriority::Realtime => debug!("Setting thread priority to Realtime"),
        }
    }
}

use native_impl::*;

pub struct ThreadManagerConfig {
    profile_handle: Option<fn(&ProfileFrame)>,
}

impl ThreadManagerConfig {
    pub fn new(profile_handle: Option<fn(&ProfileFrame)>) -> Self {
        ThreadManagerConfig { profile_handle }
    }
}

impl Default for ThreadManagerConfig {
    fn default() -> Self {
        ThreadManagerConfig {
            profile_handle: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ThreadPriority {
    Low,
    Normal,
    High,
    Realtime,
}

pub struct ProfileThreadFrame {
    pub name: String,
    pub running: bool,
    pub cpu_utilization: f32,
}

pub struct ProfileFrame {
    pub threads: Vec<ProfileThreadFrame>,
}

#[allow(dead_code)]
struct ThreadWrapper<'scope> {
    join: ScopedJoinHandle<'scope, ()>,
    native_id: NativeId,
    native_data: NativeData,
    name: String,
}

pub struct ThreadManager<'scope, 'env: 'scope> {
    threads: Arc<Mutex<Vec<ThreadWrapper<'scope>>>>,
    scope: &'scope Scope<'scope, 'env>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadError {
    JoinError(String),
    SpawnError(String),
}

impl Display for ThreadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreadError::JoinError(msg) => write!(f, "Thread join error: {}", msg),
            ThreadError::SpawnError(msg) => write!(f, "Thread spawn error: {}", msg),
        }
    }
}

impl std::error::Error for ThreadError {}

struct Feedback {
    native_id: (Condvar, Mutex<NativeId>),
}

impl Feedback {
    fn new() -> Self {
        Self {
            native_id: (Condvar::new(), Mutex::new(0)),
        }
    }

    fn post(&self, native_id: NativeId) {
        let (condvar, mutex) = &self.native_id;
        let mut id = mutex.lock().unwrap();
        *id = native_id;
        condvar.notify_one();
    }

    fn wait(&self) -> NativeId {
        let (condvar, mutex) = &self.native_id;
        let mut id = mutex.lock().unwrap();
        while *id == 0 {
            id = condvar.wait(id).unwrap();
        }
        let native_id = *id;
        *id = 0; // Reset for future use
        native_id
    }
}

fn join_all(threads: Arc<Mutex<Vec<ThreadWrapper>>>, _: Option<fn(&ProfileFrame)>) {
    // TODO: Implement profiling logic if needed
    let mut threads_guard = threads.lock().unwrap();
    for thread in threads_guard.drain(..) {
        if let Err(e) = thread.join.join() {
            eprintln!("Thread '{}' panicked: {:?}", thread.name, e);
        }
    }
}

pub fn scoped<'env, F>(cfg: ThreadManagerConfig, f: F) -> Result<(), ThreadError>
where
    F: for<'scope> FnOnce(ThreadManager<'scope, 'env>) -> (),
{
    thread::scope(|s| {
        let threads = Arc::new(Mutex::new(Vec::new()));
        let manager = ThreadManager {
            threads: Arc::clone(&threads),
            scope: s,
        };

        f(manager);

        join_all(Arc::clone(&threads), cfg.profile_handle);
    });

    Ok(())
}

impl<'scope, 'env: 'scope> ThreadManager<'scope, 'env> {
    pub fn spawn<F>(
        &self,
        name: String,
        priority: ThreadPriority,
        thread_fn: F,
    ) -> Result<(), ThreadError>
    where
        F: FnOnce() + Send + 'scope,
    {
        let feedback = Arc::new(Feedback::new());
        let feedback_clone = Arc::clone(&feedback);
        let name_clone = name.clone();

        let handle = thread::Builder::new()
            .name(name.clone())
            .spawn_scoped(self.scope, move || {
                set_my_priority(priority);
                let native_id = get_thread_native_id();
                feedback_clone.post(native_id);

                info!("Thread {} started", name_clone);
                thread_fn();
                info!("Thread {} finished", name_clone);
            })
            .map_err(|e| {
                ThreadError::SpawnError(format!("Failed to spawn thread {}: {}", name, e))
            })?;

        let native_id = feedback.wait();
        let thread_wrapper = ThreadWrapper {
            join: handle,
            native_id,
            native_data: NativeData::default(),
            name,
        };

        self.threads.lock().unwrap().push(thread_wrapper);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_scope() {
        let mut a = vec![1, 2, 3];
        let mut x = 0;

        thread::scope(|s| {
            s.spawn(|| {
                println!("hello from the first scoped thread");
                // We can borrow `a` here.
                dbg!(&a);
            });
            s.spawn(|| {
                println!("hello from the second scoped thread");
                // We can even mutably borrow `x` here,
                // because no other threads are using it.
                x += a[0] + a[2];
            });
            println!("hello from the main thread");
        });

        // After the scope, we can modify and access our variables again:
        a.push(4);
        assert_eq!(x, a.len());
    }

    #[test]
    fn thread_manager_scope() {
        let mut a = vec![1, 2, 3];
        let mut x = 0;

        let cfg = ThreadManagerConfig::default();
        let _ = scoped(cfg, |mgr| {
            let _ = mgr.spawn("Thread1".to_string(), ThreadPriority::Low, || {
                println!("hello from the first scoped thread");
                dbg!(&a);
            });

            let _ = mgr.spawn("Thread2".to_string(), ThreadPriority::Low, || {
                println!("hello from the second scoped thread");
                x += a[0] + a[2];
            });
        });

        a.push(4);
        assert_eq!(x, a.len());
    }

    #[test]
    fn thread_manager_scope_by_reference() {
        struct Spawner1<'r, 'scope, 'env: 'scope> {
            manager: &'r ThreadManager<'scope, 'env>,
        }
        impl<'r, 'scope, 'env: 'scope> Spawner1<'r, 'scope, 'env> {
            fn new(manager: &'r ThreadManager<'scope, 'env>) -> Self {
                Spawner1 { manager }
            }

            fn spawn(&self) {
                let _ = self
                    .manager
                    .spawn("Thead1".to_string(), ThreadPriority::Low, || {
                        println!("hello from the first scoped thread");
                    });
            }
        }

        struct Spawner2<'r, 'scope, 'env: 'scope> {
            manager: &'r ThreadManager<'scope, 'env>,
        }
        impl<'r, 'scope, 'env: 'scope> Spawner2<'r, 'scope, 'env> {
            fn new(manager: &'r ThreadManager<'scope, 'env>) -> Self {
                Spawner2 { manager }
            }

            fn spawn(&self) {
                let _ = self
                    .manager
                    .spawn("Thead2".to_string(), ThreadPriority::Low, || {
                        println!("hello from the second scoped thread");
                    });
            }
        }

        let cfg = ThreadManagerConfig::default();
        let _ = scoped(cfg, |mgr| {
            let spawner1 = Spawner1::new(&mgr);
            let spawner2 = Spawner2::new(&mgr);

            spawner1.spawn();
            spawner2.spawn();
        });
    }
}
