use crate::gl::bindings;

pub struct GlVersion {
    pub major: u32,
    pub minor: u32,
}

impl std::fmt::Display for GlVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

pub(crate) unsafe fn get_version() -> Option<GlVersion> {
    let mut major = 0;
    let mut minor = 0;
    bindings::GetIntegerv(bindings::MAJOR_VERSION, &mut major);
    bindings::GetIntegerv(bindings::MINOR_VERSION, &mut minor);
    if major > 0 && minor > 0 {
        Some(GlVersion {
            major: major as u32,
            minor: minor as u32,
        })
    } else {
        None
    }
}

pub(crate) unsafe fn get_vendor() -> Option<String> {
    let vendor_ptr = bindings::GetString(bindings::VENDOR);
    if !vendor_ptr.is_null() {
        Some(std::ffi::CStr::from_ptr(vendor_ptr as *const i8)
            .to_string_lossy()
            .into_owned())
    } else {
        None
    }
}

pub(crate) unsafe fn get_renderer() -> Option<String> {
    let renderer_ptr = bindings::GetString(bindings::RENDERER);
    if !renderer_ptr.is_null() {
        Some(std::ffi::CStr::from_ptr(renderer_ptr as *const i8)
            .to_string_lossy()
            .into_owned())
    } else {
        None
    }
}

pub(crate) unsafe fn gl_get_extensions() -> Vec<String> {
    let extensions_ptr = bindings::GetString(bindings::EXTENSIONS);
    if extensions_ptr.is_null() {
        return Vec::new();
    }

    let extensions_str = std::ffi::CStr::from_ptr(extensions_ptr as *const i8)
        .to_string_lossy()
        .into_owned();

    extensions_str
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

