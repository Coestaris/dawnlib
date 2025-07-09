use log::{debug, info};
use std::fmt::{Display, Formatter};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

#[cfg(target_os = "linux")]
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
mod native_impl {
    use log::debug;
    use crate::threads::ThreadPriority;

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
mod native_impl {
    use log::debug;
    use crate::threads::ThreadPriority;

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
    pub profile_handle: Option<fn(&ProfileFrame)>,
}

#[derive(Debug, Clone, Copy)]
pub enum ThreadPriority {
    Low,
    Normal,
    High,
    Realtime,
}

struct ThreadWrapper {
    join: JoinHandle<()>,
    native_id: NativeId,
    native_data: NativeData,
    name: String,
}

pub struct ProfileThreadFrame {
    pub name: String,
    pub running: bool,
    pub cpu_utilization: f32,
}

pub struct ProfileFrame {
    pub threads: Vec<ProfileThreadFrame>,
}

pub struct ThreadManager {
    threads: Mutex<Vec<ThreadWrapper>>,
    profile_handle: Option<fn(&ProfileFrame)>,
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

impl ThreadManager {
    pub fn new(config: ThreadManagerConfig) -> Self {
        Self {
            threads: Mutex::new(Vec::new()),
            profile_handle: config.profile_handle,
        }
    }

    pub fn spawn(
        &self,
        name: String,
        priority: ThreadPriority,
        thread_fn: impl FnOnce() + Send + 'static,
    ) -> Result<(), ThreadError> {
        let feedback = Arc::new(Feedback::new());
        let feedback_clone = Arc::clone(&feedback);
        let name_clone = name.clone();
        let handle = std::thread::Builder::new()
            .name(name.clone())
            .spawn(move || {
                // Set thread priority
                set_my_priority(priority);

                // Get native thread ID
                let native_id = get_thread_native_id();
                feedback_clone.post(native_id);

                info!("Thread {} started (priority: {:?})", name_clone, priority);

                // Run the thread function
                thread_fn();

                info!("Thread {} finished execution", name_clone);
            })
            .map_err(|e| {
                ThreadError::SpawnError(format!("Failed to spawn thread {}: {}", name, e))
            })?;

        // Wait for the thread to be ready and get its native ID
        let native_id = feedback.wait();
        debug!("Thread {} has native ID: {}", name, native_id);

        // Store the thread information
        let thread_wrapper = ThreadWrapper {
            join: handle,
            native_data: NativeData::default(),
            native_id,
            name,
        };
        let mut threads = self.threads.lock().unwrap();
        threads.push(thread_wrapper);

        Ok(())
    }

    pub fn join_all(&self) {
        info!("Joining all threads");

        // Join all threads and clear the list
        // While waiting for threads to finish, print threads count and their cpu utilization
        loop {
            let mut threads = self.threads.lock().unwrap();
            let mut all_finished = true;
            for thread in &*threads {
                if !thread.join.is_finished() {
                    all_finished = false;
                    break;
                }
            }

            if all_finished {
                break;
            }

            // Collect profile information
            if let Some(profile_handle) = &self.profile_handle {
                let mut frames = Vec::new();
                for thread in &mut *threads {
                    if thread.join.is_finished() {
                        continue; // Skip finished threads
                    }

                    let cpu_utilization =
                        get_cpu_utilization(&mut thread.native_data, thread.native_id);
                    frames.push(ProfileThreadFrame {
                        name: thread.name.clone(),
                        running: !thread.join.is_finished(),
                        cpu_utilization,
                    });
                }

                let profile_frame = ProfileFrame { threads: frames };
                profile_handle(&profile_frame);
            }

            // Release the lock before sleeping
            drop(threads);

            // Wait for a short duration to avoid busy waiting
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }

        info!("All threads have finished execution");
    }
}
