use crate::gl::bindings;
use crate::gl::bindings::types::GLuint;
use crate::gl::entities::shader::Shader;
use crate::passes::events::PassEventTrait;
use log::debug;
use dawn_assets::AssetCastable;
use dawn_assets::raw::ShaderAssetRaw;

#[derive(Debug)]
// RAII wrapper for OpenGL shader program
pub struct ShaderProgram {
    id: GLuint,
}

pub struct ShaderProgramBind<'a> {
    program: &'a ShaderProgram,
}

impl<'a> ShaderProgramBind<'a> {
    pub fn new(program: &'a ShaderProgram) -> Self {
        unsafe {
            bindings::UseProgram(program.id);
        }
        Self { program }
    }

    #[inline(always)]
    pub fn set_uniform<T: UniformTarget>(&self, location: UniformLocation, value: T) {
        T::set_uniform(location, value);
    }
}

impl Drop for ShaderProgramBind<'_> {
    fn drop(&mut self) {
        unsafe {
            bindings::UseProgram(0);
        }
    }
}

pub type UniformLocation = GLuint;

impl AssetCastable for ShaderProgram {}

pub trait UniformTarget {
    fn set_uniform(location: UniformLocation, value: Self);
}

macro_rules! define_target(
    ($type:ty, $binding:expr) => {
        impl UniformTarget for $type {
            #[inline(always)]
            fn set_uniform(location: UniformLocation, value: Self) {
                unsafe {
                    $binding(location as GLint, value);
                }
            }
        }
    }
);

#[rustfmt::skip]
mod targets {
    use crate::gl::entities::shader_program::UniformLocation;
    use crate::gl::entities::shader_program::UniformTarget;
    use crate::gl::bindings::types::GLint;
    use glam::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
    use crate::gl::bindings;

    define_target!(u32, |l, v| bindings::Uniform1ui(l, v));
    define_target!(i32, |l, v| bindings::Uniform1i(l, v));
    define_target!(f32, |l, v| bindings::Uniform1f(l, v));
    define_target!(Vec2, |l, v: Vec2| bindings::Uniform2f(l, v.x, v.y));
    define_target!(Vec3, |l, v: Vec3| bindings::Uniform3f(l, v.x, v.y, v.z));
    define_target!(Vec4, |l, v: Vec4| bindings::Uniform4f(l, v.x, v.y, v.z, v.w));
    define_target!(UVec2, |l, v: UVec2| bindings::Uniform2ui(l, v.x, v.y));
    define_target!(UVec3, |l, v: UVec3| bindings::Uniform3ui(l, v.x, v.y, v.z));
    define_target!(UVec4, |l, v: UVec4| bindings::Uniform4ui(l, v.x, v.y, v.z, v.w));
    define_target!(IVec2, |l, v: IVec2| bindings::Uniform2i(l, v.x, v.y));
    define_target!(IVec3, |l, v: IVec3| bindings::Uniform3i(l, v.x, v.y, v.z));
    define_target!(IVec4, |l, v: IVec4| bindings::Uniform4i(l, v.x, v.y, v.z, v.w));
    define_target!(bool, |l, v| bindings::Uniform1i(l, if v { 1 } else { 0 }));
    define_target!(glam::Mat2, |l, v: glam::Mat2| { bindings::UniformMatrix2fv(l, 1, bindings::FALSE, v.as_ref().as_ptr()) });
    define_target!(glam::Mat3, |l, v: glam::Mat3| { bindings::UniformMatrix3fv(l, 1, bindings::FALSE, v.as_ref().as_ptr()) });
    define_target!(glam::Mat4, |l, v: glam::Mat4| { bindings::UniformMatrix4fv(l, 1, bindings::FALSE, v.as_ref().as_ptr()) });
    define_target!(glam::Quat, |l, v: glam::Quat| { bindings::Uniform4fv(l, 1, v.as_ref().as_ptr()) });
}

pub use targets::*;

impl ShaderProgram {
    pub(crate) fn from_raw<E: PassEventTrait>(
        raw: &ShaderAssetRaw,
    ) -> Result<ShaderProgram, String> {
        // TODO: Cache the compilation result
        // TODO: Try load SPIRV instead of compiling from source
        let program = ShaderProgram::new().ok_or("Failed to create shader program")?;

        for (source_type, source) in &raw.sources {
            let shader = Shader::new(*source_type)?;
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

        debug!("Allocated shader program ID: {}", program.id);
        Ok(program)
    }

    fn new() -> Option<ShaderProgram> {
        debug!("Creating program");
        let id = unsafe { bindings::CreateProgram() };
        if id == 0 {
            return None;
        }
        Some(ShaderProgram { id })
    }

    fn attach_shader(&self, shader: Shader) {
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
    #[must_use]
    pub fn bind(&self) -> ShaderProgramBind<'_> {
        ShaderProgramBind::new(self)
    }

    #[inline(always)]
    pub fn get_uniform_location(&self, name: &str) -> Result<UniformLocation, String> {
        let c_name = std::ffi::CString::new(name)
            .map_err(|e| format!("Failed to create CString for uniform name: {}", e))?;
        let location = unsafe { bindings::GetUniformLocation(self.id, c_name.as_ptr()) };
        if location == -1 {
            Err(format!("Uniform '{}' not found in shader program", name))
        } else {
            Ok(location as UniformLocation)
        }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        debug!("Dropping shader program ID: {}", self.id);
        unsafe {
            bindings::DeleteProgram(self.id);
        }
    }
}
