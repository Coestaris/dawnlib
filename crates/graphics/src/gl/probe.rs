use glow::HasContext;

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
pub(crate) unsafe fn get_version(gl: &glow::Context) -> Option<GlVersion> {
    let mut major = gl.get_parameter_i32(glow::MAJOR_VERSION);
    let mut minor = gl.get_parameter_i32(glow::MINOR_VERSION);
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
pub(crate) unsafe fn get_vendor(gl: &glow::Context) -> Option<String> {
    Some(gl.get_parameter_string(glow::VENDOR))
}

#[allow(unused)]
pub(crate) unsafe fn get_renderer(gl: &glow::Context) -> Option<String> {
    Some(gl.get_parameter_string(glow::RENDERER))
}

#[allow(unused)]
pub(crate) unsafe fn get_shading_language_version(
    gl: &glow::Context,
) -> Option<ShadingLanguageVersion> {
    let version = gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION);

    // Split by dots and spaces
    let parts: Vec<&str> = version.split(['.', ' ']).collect();
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
}

#[allow(unused)]
pub(crate) unsafe fn get_binary_formats(gl: &glow::Context) -> Vec<i32> {
    let mut num_formats = gl.get_parameter_i32(glow::NUM_SHADER_BINARY_FORMATS);

    let mut result = Vec::new();
    for _ in 0..num_formats {
        result.push(gl.get_parameter_i32(glow::SHADER_BINARY_FORMATS));
    }

    result
}

#[allow(unused)]
pub(crate) unsafe fn get_extensions(gl: &glow::Context) -> Vec<String> {
    let extensions = gl.get_parameter_string(glow::EXTENSIONS);
    extensions
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

pub(crate) unsafe fn get_depth_bits(gl: &glow::Context) -> Option<u32> {
    let depth_bits = gl.get_parameter_i32(glow::DEPTH_BITS);
    if depth_bits >= 0 {
        Some(depth_bits as u32)
    } else {
        None
    }
}
