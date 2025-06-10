use std::ffi::{c_char, CStr};

pub const VALIDATION_LAYER_NAME: *const c_char =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") }.as_ptr()
        as *const c_char;
pub const SURFACE_EXTENSION_NAME: *const c_char =
    unsafe { ash::khr::surface::NAME.as_ptr() as *const c_char };
pub const SWAPCHAIN_EXTENSION_NAME: *const c_char =
    unsafe { ash::khr::swapchain::NAME.as_ptr() as *const c_char };
pub const DEBUG_UTILS_EXTENSION_NAME: *const c_char =
    unsafe { ash::ext::debug_utils::NAME.as_ptr() as *const c_char };
pub const DEBUG_REPORT_EXTENSION_NAME: *const c_char =
    unsafe { ash::ext::debug_report::NAME.as_ptr() as *const c_char };

pub fn get_required_layers() -> Vec<*const c_char> {
    vec![]
}

pub fn get_wanted_layers() -> Vec<*const c_char> {
    vec![VALIDATION_LAYER_NAME]
}

pub fn get_required_instance_extensions() -> Vec<*const c_char> {
    vec![SURFACE_EXTENSION_NAME]
}

pub fn get_wanted_instance_extensions() -> Vec<*const c_char> {
    if cfg!(debug_assertions) {
        vec![DEBUG_UTILS_EXTENSION_NAME, DEBUG_REPORT_EXTENSION_NAME]
    } else {
        vec![]
    }
}

pub fn get_required_device_extensions() -> Vec<*const c_char> {
    vec![SWAPCHAIN_EXTENSION_NAME]
}

pub fn get_wanted_device_extensions() -> Vec<*const c_char> {
    vec![]
}
