use libc::localtime_s;
use std::ffi::{c_char, CStr};
use std::mem;
use std::ptr::{addr_of_mut};

pub(crate) fn contains(vec: &Vec<*const c_char>, item: *const c_char) -> bool {
    vec.iter()
        .any(|&x| unsafe { CStr::from_ptr(x).eq(&CStr::from_ptr(item)) })
}

/* Use a simple format instead of something like strftime,
 * to avoid unnecessary complexity, and to not extend the
 * dependency tree with a crate that provides it. */
pub(crate) fn format_now(format: &str) -> Option<String> {
    /* Get tm-like representation of the current time */
    let system_time = std::time::SystemTime::now();
    let duration = system_time.duration_since(std::time::UNIX_EPOCH).ok()?;

    let tm = unsafe {
        let datetime = libc::time_t::try_from(duration.as_secs()).ok()?;
        let mut ret = mem::zeroed();
        if (localtime_s(addr_of_mut!(ret), &datetime) == 0) {
            ret
        } else {
            return None;
        }
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
