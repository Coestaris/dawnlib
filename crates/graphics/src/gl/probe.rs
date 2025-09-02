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

pub struct ShadingLanguageVersion {
    pub major: u32,
    pub minor: u32,
    pub release: u32,
    pub vendor_specific: String,
}

impl std::fmt::Display for ShadingLanguageVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{} ({})",
            self.major, self.minor, self.release, self.vendor_specific
        )
    }
}

#[allow(unused)]
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

#[allow(unused)]
pub(crate) unsafe fn get_vendor() -> Option<String> {
    let vendor_ptr = bindings::GetString(bindings::VENDOR);
    if !vendor_ptr.is_null() {
        Some(
            std::ffi::CStr::from_ptr(vendor_ptr as *const i8)
                .to_string_lossy()
                .into_owned(),
        )
    } else {
        None
    }
}

#[allow(unused)]
pub(crate) unsafe fn get_renderer() -> Option<String> {
    let renderer_ptr = bindings::GetString(bindings::RENDERER);
    if !renderer_ptr.is_null() {
        Some(
            std::ffi::CStr::from_ptr(renderer_ptr as *const i8)
                .to_string_lossy()
                .into_owned(),
        )
    } else {
        None
    }
}

#[allow(unused)]
pub(crate) unsafe fn get_shading_language_version() -> Option<ShadingLanguageVersion> {
    let version_ptr = bindings::GetString(bindings::SHADING_LANGUAGE_VERSION);
    if !version_ptr.is_null() {
        let version_str = std::ffi::CStr::from_ptr(version_ptr as *const i8)
            .to_string_lossy()
            .into_owned();

        // Split by dots and spaces
        let parts: Vec<&str> = version_str.split(['.', ' ']).collect();
        if parts.len() >= 2 {
            let major = parts[0].parse::<u32>().unwrap_or(0);
            let minor = parts[1].parse::<u32>().unwrap_or(0);

            let mut vendor_start = 1;
            let release = if parts.len() > 2 {
                vendor_start = 2;
                parts[2].parse::<u32>().unwrap_or(0)
            } else {
                0
            };
            let vendor_specific = parts[vendor_start..].join(" ");

            Some(ShadingLanguageVersion {
                major,
                minor,
                release,
                vendor_specific,
            })
        } else {
            None
        }
    } else {
        None
    }
}

#[allow(unused)]
pub(crate) unsafe fn get_binary_formats() -> Vec<u32> {
    let mut num_formats = 0;
    bindings::GetIntegerv(bindings::NUM_SHADER_BINARY_FORMATS, &mut num_formats);
    if num_formats <= 0 {
        return Vec::new();
    }

    let mut formats = vec![0; num_formats as usize];
    bindings::GetIntegerv(bindings::SHADER_BINARY_FORMATS, formats.as_mut_ptr());

    formats.into_iter().map(|f| f as u32).collect()
}

#[allow(unused)]
pub(crate) unsafe fn get_extensions() -> Vec<String> {
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

pub(crate) unsafe fn get_depth_bits() -> Option<u32> {
    let mut depth_bits = 0;
    bindings::GetIntegerv(bindings::DEPTH_BITS, &mut depth_bits);
    if depth_bits > 0 {
        Some(depth_bits as u32)
    } else {
        None
    }
}
