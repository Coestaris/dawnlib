use std::thread::JoinHandle;

struct ThreadManagerConfig {
    profile_handle: Option<fn(&ProfileFrame)>,
}

struct ThreadManager {
    threads: Vec<JoinHandle<()>>,
}

struct ProfileThreadFrame {
    thread_id: usize,
    name: String,
    cpu_utilization: f32,
}

struct ProfileFrame {
    threads: Vec<ProfileThreadFrame>,
}

impl ThreadManager {
    pub fn new() -> Self {
        Self {
            threads: Vec::new(),
        }
    }

    pub fn spawn(&mut self, name: String, thread_fn: impl FnOnce() + Send + 'static) {
        let handle = std::thread::Builder::new()
            .name(name.clone())
            .spawn(thread_fn)
            .expect("Failed to spawn thread");

        self.threads.push(handle);
    }

    pub fn join_all(&mut self) {
        // Join all threads and clear the list
        // While waiting for threads to finish, print threads count and their cpu utilization
        loop {
            let mut all_finished = true;
            for handle in &self.threads {
                if !handle.is_finished() {
                    all_finished = false;
                    break;
                }
            }

            if all_finished {
                break;
            }
            
            
        }

        self.threads.clear();
    }
}
