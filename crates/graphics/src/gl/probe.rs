use glow::HasContext;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct GlVersion {
    pub major: u32,
    pub minor: u32,
}

impl std::fmt::Display for GlVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct OpenGLInfo {
    pub version: glow::Version,
    pub renderer: String,
    pub shading_language_version: Option<ShadingLanguageVersion>,
    pub binary_formats: HashSet<i32>,
    pub extensions: HashSet<String>,
    pub depth_bits: Option<u32>,
    pub stencil_bits: Option<u32>,
    pub limits: OpenGLLimits,
}

#[derive(Debug, Clone)]
pub struct TextureOpenGLInfo {
    pub max_texture_size: u32,
    pub max_texture_image_units: u32,
    pub max_combined_texture_image_units: u32,
    pub max_cube_map_texture_size: u32,
}

#[derive(Debug, Clone)]
pub struct BufferOpenGLInfo {
    pub max_vertex_attribs: u32,
    pub max_vertex_uniform_vectors: u32,
    pub max_fragment_uniform_vectors: u32,
    pub max_varying_vectors: u32,
    pub max_combined_uniform_blocks: u32,
    pub max_uniform_buffer_bindings: u32,
    pub max_uniform_block_size: u32,
    pub uniform_buffer_offset_alignment: u32,
}

#[derive(Debug, Clone)]
pub struct ShaderOpenGLInfo {
    pub max_vertex_shader_storage_blocks: u32,
    pub max_fragment_shader_storage_blocks: u32,
    pub max_combined_shader_storage_blocks: u32,
    pub max_shader_storage_buffer_bindings: u32,
    pub shader_storage_buffer_offset_alignment: u32,
}

#[derive(Debug, Clone)]
pub struct FramebufferOpenGLInfo {
    pub max_color_attachments: u32,
    pub max_draw_buffers: u32,
}

#[derive(Debug, Clone)]
pub struct OpenGLLimits {
    pub texture: TextureOpenGLInfo,
    pub buffer: BufferOpenGLInfo,
    pub shader: ShaderOpenGLInfo,
    pub framebuffer: FramebufferOpenGLInfo,
}

impl OpenGLInfo {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        Self {
            version: gl.version().clone(),
            renderer: Self::get_renderer(gl),
            shading_language_version: Self::get_shading_language_version(gl),
            binary_formats: Self::get_binary_formats(gl),
            extensions: Self::get_extensions(gl),
            depth_bits: Self::get_depth_bits(gl),
            stencil_bits: Self::get_stencil_bits(gl),
            limits: OpenGLLimits::new(gl),
        }
    }

    unsafe fn get_renderer(gl: &glow::Context) -> String {
        gl.get_parameter_string(glow::RENDERER)
    }

    unsafe fn get_shading_language_version(gl: &glow::Context) -> Option<ShadingLanguageVersion> {
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

    unsafe fn get_binary_formats(gl: &glow::Context) -> HashSet<i32> {
        let num_formats = gl.get_parameter_i32(glow::NUM_SHADER_BINARY_FORMATS);

        let mut result = HashSet::new();
        for _ in 0..num_formats {
            result.insert(gl.get_parameter_i32(glow::SHADER_BINARY_FORMATS));
        }

        result
    }

    unsafe fn get_extensions(gl: &glow::Context) -> HashSet<String> {
        gl.supported_extensions().clone()
    }

    unsafe fn get_depth_bits(gl: &glow::Context) -> Option<u32> {
        let depth_bits = gl.get_parameter_i32(glow::DEPTH_BITS);
        if depth_bits >= 0 {
            Some(depth_bits as u32)
        } else {
            None
        }
    }

    unsafe fn get_stencil_bits(gl: &glow::Context) -> Option<u32> {
        let stencil_bits = gl.get_parameter_i32(glow::STENCIL_BITS);
        if stencil_bits >= 0 {
            Some(stencil_bits as u32)
        } else {
            None
        }
    }
}

impl TextureOpenGLInfo {
    unsafe fn new(gl: &glow::Context) -> Self {
        Self {
            max_texture_size: gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) as u32,
            max_texture_image_units: gl.get_parameter_i32(glow::MAX_TEXTURE_IMAGE_UNITS) as u32,
            max_combined_texture_image_units: gl
                .get_parameter_i32(glow::MAX_COMBINED_TEXTURE_IMAGE_UNITS)
                as u32,
            max_cube_map_texture_size: gl.get_parameter_i32(glow::MAX_CUBE_MAP_TEXTURE_SIZE) as u32,
        }
    }
}

impl BufferOpenGLInfo {
    unsafe fn new(gl: &glow::Context) -> Self {
        Self {
            max_vertex_attribs: gl.get_parameter_i32(glow::MAX_VERTEX_ATTRIBS) as u32,
            max_vertex_uniform_vectors: gl.get_parameter_i32(glow::MAX_VERTEX_UNIFORM_VECTORS)
                as u32,
            max_fragment_uniform_vectors: gl.get_parameter_i32(glow::MAX_FRAGMENT_UNIFORM_VECTORS)
                as u32,
            max_varying_vectors: gl.get_parameter_i32(glow::MAX_VARYING_VECTORS) as u32,
            max_combined_uniform_blocks: gl.get_parameter_i32(glow::MAX_COMBINED_UNIFORM_BLOCKS)
                as u32,
            max_uniform_buffer_bindings: gl.get_parameter_i32(glow::MAX_UNIFORM_BUFFER_BINDINGS)
                as u32,
            max_uniform_block_size: gl.get_parameter_i32(glow::MAX_UNIFORM_BLOCK_SIZE) as u32,
            uniform_buffer_offset_alignment: gl
                .get_parameter_i32(glow::UNIFORM_BUFFER_OFFSET_ALIGNMENT)
                as u32,
        }
    }
}

impl FramebufferOpenGLInfo {
    unsafe fn new(gl: &glow::Context) -> Self {
        Self {
            max_color_attachments: gl.get_parameter_i32(glow::MAX_COLOR_ATTACHMENTS) as u32,
            max_draw_buffers: gl.get_parameter_i32(glow::MAX_DRAW_BUFFERS) as u32,
        }
    }
}

impl ShaderOpenGLInfo {
    unsafe fn new(gl: &glow::Context) -> Self {
        Self {
            max_vertex_shader_storage_blocks: gl
                .get_parameter_i32(glow::MAX_VERTEX_SHADER_STORAGE_BLOCKS)
                as u32,
            max_fragment_shader_storage_blocks: gl
                .get_parameter_i32(glow::MAX_FRAGMENT_SHADER_STORAGE_BLOCKS)
                as u32,
            max_combined_shader_storage_blocks: gl
                .get_parameter_i32(glow::MAX_COMBINED_SHADER_STORAGE_BLOCKS)
                as u32,
            max_shader_storage_buffer_bindings: gl
                .get_parameter_i32(glow::MAX_SHADER_STORAGE_BUFFER_BINDINGS)
                as u32,
            shader_storage_buffer_offset_alignment: gl
                .get_parameter_i32(glow::SHADER_STORAGE_BUFFER_OFFSET_ALIGNMENT)
                as u32,
        }
    }
}

impl OpenGLLimits {
    unsafe fn new(gl: &glow::Context) -> Self {
        Self {
            texture: TextureOpenGLInfo::new(gl),
            buffer: BufferOpenGLInfo::new(gl),
            shader: ShaderOpenGLInfo::new(gl),
            framebuffer: FramebufferOpenGLInfo::new(gl),
        }
    }
}
