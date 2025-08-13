use crate::gl::bindings;
use crate::gl::bindings::types::GLuint;
use glam::Vec2;
use log::{debug, warn};
use yage2_core::assets::raw::{ShaderAssetRaw, ShaderSourceType};
use yage2_core::assets::AssetCastable;

#[derive(Debug)]
// RAII wrapper for OpenGL shader program
pub struct Shader {
    id: GLuint,
}

impl AssetCastable for Shader {}

pub trait UniformTarget {
    fn set_uniform(location: GLuint, value: Self);
}

impl UniformTarget for Vec2 {
    fn set_uniform(location: GLuint, value: Self) {
        unsafe {
            bindings::Uniform2fv(location as i32, 1, value.as_ref() as *const f32);
        }
    }
}

impl UniformTarget for i32 {
    fn set_uniform(location: GLuint, value: Self) {
        unsafe {
            bindings::Uniform1i(location as i32, value);
        }
    }
}

impl Shader {
    pub(crate) fn from_raw(raw: &ShaderAssetRaw) -> Result<Shader, String> {
        // TODO: Cache the compilation result
        // TODO: Try load SPIRV insteaad of compiling from source
        let program = Shader::new().ok_or("Failed to create shader program")?;

        for (source_type, source) in &raw.sources {
            let shader = ShaderPass::new(*source_type).ok_or("Failed to create shader")?;
            let source = String::from_utf8(source.clone())
                .map_err(|e| format!("Failed to convert shader source to UTF-8: {}", e))?;
            shader
                .set_source(source)
                .map_err(|e| format!("Failed to set shader source: {}", e))?;
            shader
                .compile()
                .map_err(|e| format!("Failed to compile shader: {}", e))?;
            program.attach_shader(shader);
        }

        program
            .link()
            .map_err(|e| format!("Failed to link shader program: {}", e))?;

        debug!("Shader program created with ID: {}", program.id);
        Ok(program)
    }

    fn new() -> Option<Shader> {
        debug!("Creating program");
        let id = unsafe { bindings::CreateProgram() };
        if id == 0 {
            return None;
        }
        Some(Shader { id })
    }

    fn attach_shader(&self, shader: ShaderPass) {
        unsafe {
            bindings::AttachShader(self.id, shader.id());
        }
    }

    fn link(&self) -> Result<(), String> {
        unsafe {
            bindings::LinkProgram(self.id);
            let mut status = 0;
            bindings::GetProgramiv(self.id, bindings::LINK_STATUS, &mut status);
            if status == 0 {
                let mut log_length = 0;
                bindings::GetProgramiv(self.id, bindings::INFO_LOG_LENGTH, &mut log_length);
                let mut log = vec![0; log_length as usize];
                bindings::GetProgramInfoLog(
                    self.id,
                    log_length,
                    std::ptr::null_mut(),
                    log.as_mut_ptr() as *mut i8,
                );
                return Err(String::from_utf8_lossy(&log).to_string());
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub fn id(&self) -> GLuint {
        self.id
    }

    #[inline(always)]
    pub fn use_program(&self) {
        unsafe {
            bindings::UseProgram(self.id);
        }
    }

    #[inline(always)]
    pub fn get_uniform_location(&self, name: &str) -> Result<GLuint, String> {
        let c_name = std::ffi::CString::new(name)
            .map_err(|e| format!("Failed to create CString for uniform name: {}", e))?;
        let location = unsafe { bindings::GetUniformLocation(self.id, c_name.as_ptr()) };
        if location == -1 {
            Err(format!("Uniform '{}' not found in shader program", name))
        } else {
            Ok(location as GLuint)
        }
    }

    pub fn set_uniform<T: UniformTarget>(&self, location: GLuint, value: T) {
        T::set_uniform(location, value);
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        debug!("Deleting shader program with ID: {}", self.id);
        unsafe {
            bindings::DeleteProgram(self.id);
        }
    }
}

// RAII wrapper for OpenGL shader
struct ShaderPass {
    id: GLuint,
}

impl ShaderPass {
    fn new(source_type: ShaderSourceType) -> Option<ShaderPass> {
        debug!("Creating shader of type: {:?}", source_type);
        let gl_type = match source_type {
            ShaderSourceType::Vertex => bindings::VERTEX_SHADER,
            ShaderSourceType::Fragment => bindings::FRAGMENT_SHADER,
            ShaderSourceType::Geometry => bindings::GEOMETRY_SHADER,
            ShaderSourceType::Compute => bindings::COMPUTE_SHADER,
            ShaderSourceType::TessellationControl => bindings::TESS_CONTROL_SHADER,
            _ => {
                warn!("Unsupported shader source type: {:?}", source_type);
                return None;
            }
        };

        let id = unsafe { bindings::CreateShader(gl_type) };
        if id == 0 {
            return None;
        }
        Some(ShaderPass { id })
    }

    pub fn id(&self) -> GLuint {
        self.id
    }

    pub fn set_source(&self, source: String) -> Result<(), String> {
        let c_source = std::ffi::CString::new(source)
            .map_err(|e| format!("Failed to create CString for shader source: {}", e))?;
        unsafe {
            bindings::ShaderSource(self.id, 1, &c_source.as_ptr(), std::ptr::null());
        }
        Ok(())
    }

    pub fn compile(&self) -> Result<(), String> {
        unsafe {
            bindings::CompileShader(self.id);
            let mut status = 0;
            bindings::GetShaderiv(self.id, bindings::COMPILE_STATUS, &mut status);
            if status == 0 {
                let mut log_length = 0;
                bindings::GetShaderiv(self.id, bindings::INFO_LOG_LENGTH, &mut log_length);
                let mut log = vec![0; log_length as usize];
                bindings::GetShaderInfoLog(
                    self.id,
                    log_length,
                    std::ptr::null_mut(),
                    log.as_mut_ptr() as *mut i8,
                );
                return Err(String::from_utf8_lossy(&log).to_string());
            }
        }
        Ok(())
    }
}

impl Drop for ShaderPass {
    fn drop(&mut self) {
        debug!("Deleting shader with ID: {}", self.id);
        unsafe {
            bindings::DeleteShader(self.id);
        }
    }
}
