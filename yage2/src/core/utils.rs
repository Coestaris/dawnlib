use std::ffi::{c_char, CStr};
use std::mem;
use std::ptr::addr_of_mut;

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
