use std::ffi::{c_char, CStr};
use std::mem;
use std::ptr::addr_of_mut;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

pub(crate) fn contains(vec: &Vec<*const c_char>, item: *const c_char) -> bool {
    vec.iter()
        .any(|&x| unsafe { CStr::from_ptr(x).eq(&CStr::from_ptr(item)) })
}

/* Use a simple format instead of something like strftime,
 * to avoid unnecessary complexity, and to not extend the
 * dependency tree with a crate that provides it. */
#[allow(unused_imports)]
pub fn format_now() -> Option<String> {
    /* Get tm-like representation of the current time */
    let system_time = std::time::SystemTime::now();
    let duration = system_time.duration_since(std::time::UNIX_EPOCH).ok()?;

    use libc::tm;
    let tm = unsafe {
        let datetime = libc::time_t::try_from(duration.as_secs()).ok()?;
        let mut ret = mem::zeroed();
        #[cfg(windows)]
        {
            use libc::localtime_s;
            libc::localtime_s(addr_of_mut!(ret), &datetime);
        }
        #[cfg(unix)]
        {
            use libc::localtime_r;
            libc::localtime_r(&datetime, addr_of_mut!(ret));
        }
        ret
    };

    /* Format:
     * YYYY.MM.DD HH:MM:SS.{ms} */
    Some(format!(
        "{:04}.{:02}.{:02} {:02}:{:02}:{:02}.{:03}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec,
        duration.subsec_millis()
    ))
}

/* Sync point that allows threads to synchronize at specific points.
 * Allows threads to wait for each other before proceeding. */
pub struct Rendezvous {
    mutex: Arc<Mutex<u32>>,
    condvar: Arc<Condvar>,
    max_val: u32,
}

impl Rendezvous {
    pub fn new(max_val: u32) -> Self {
        Self {
            mutex: Arc::new(Mutex::new(0)),
            condvar: Arc::new(Condvar::new()),
            max_val: 2,
        }
    }

    /* Waits at the rendezvous point until `max_val` threads have called this method.
     * If `timeout` is `Some(ms)`, the thread will wait at most that many milliseconds.
     * Returns `true` if the rendezvous was successfully reached, `false` if timed out. */
    pub fn wait(&self, timeout: Option<u64>) -> bool {
        let mut count = self.mutex.lock().unwrap();
        *count += 1;

        if *count >= self.max_val {
            // Reset count for next rendezvous point
            *count = 0;
            self.condvar.notify_all();
            true
        } else {
            let condvar = &self.condvar;
            let result = if let Some(ms) = timeout {
                let duration = Duration::from_millis(ms);
                let now = Instant::now();

                let (mut count, timeout_result) = condvar
                    .wait_timeout_while(count, duration, |c| *c < self.max_val)
                    .unwrap();

                if *count >= self.max_val {
                    // Another thread triggered the rendezvous
                    true
                } else {
                    // Timed out
                    false
                }
            } else {
                // Wait without timeout
                while *count < self.max_val {
                    count = condvar.wait(count).unwrap();
                }
                true
            };

            result
        }
    }
}
