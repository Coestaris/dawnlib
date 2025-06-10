use std::ffi::{c_char, CStr};

pub(crate) fn contains(vec: &Vec<*const c_char>, item: *const c_char) -> bool {
    vec.iter()
        .any(|&x| unsafe { CStr::from_ptr(x).eq(&CStr::from_ptr(item)) })
}
