use std::sync::Arc;
use crate::passes::events::PassEventTrait;
use dawn_assets::ir::shader::IRShader;
use dawn_assets::{AssetCastable, AssetMemoryUsage};
use glow::HasContext;
use log::debug;

#[derive(Debug)]
pub struct Program {
    gl: Arc<glow::Context>,
    inner: glow::Program,
}

pub type UniformLocation = glow::UniformLocation;
pub type UniformBlockLocation = glow::UniformLocation;

impl AssetCastable for Program {}

pub trait UniformTarget {
    fn set_uniform(gl: &glow::Context, location: &UniformLocation, value: Self);
}

macro_rules! define_target(
    ($type:ty, $binding:expr) => {
        impl UniformTarget for $type {
            #[inline(always)]
            fn set_uniform(gl: &glow::Context, location: &UniformLocation, value: Self) {
                unsafe {
                    $binding(gl, location, value);
                }
            }
        }
    }
);

#[rustfmt::skip]
mod targets {
    use crate::gl::raii::shader_program::UniformLocation;
use crate::gl::raii::shader_program::UniformTarget;
use glam::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
use glow::{Context, HasContext};

    define_target!(u32, |g: &Context, l, v| g.uniform_1_u32(Some(l), v));
    define_target!(i32, |g: &Context, l, v| g.uniform_1_i32(Some(l), v));
    define_target!(f32, |g: &Context, l, v| g.uniform_1_f32(Some(l), v));
    define_target!(Vec2, |g: &Context, l, v: Vec2| g.uniform_2_f32(Some(l), v.x, v.y));
    define_target!(Vec3, |g: &Context, l, v: Vec3| g.uniform_3_f32(Some(l), v.x, v.y, v.z));
    define_target!(Vec4, |g: &Context, l, v: Vec4| g.uniform_4_f32(Some(l), v.x, v.y, v.z, v.w));
    define_target!(UVec2, |g: &Context, l, v: UVec2| g.uniform_2_u32(Some(l), v.x, v.y));
    define_target!(UVec3, |g: &Context, l, v: UVec3| g.uniform_3_u32(Some(l), v.x, v.y, v.z));
    define_target!(UVec4, |g: &Context, l, v: UVec4| g.uniform_4_u32(Some(l), v.x, v.y, v.z, v.w));
    define_target!(IVec2, |g: &Context, l, v: IVec2| g.uniform_2_i32(Some(l), v.x, v.y));
    define_target!(IVec3, |g: &Context, l, v: IVec3| g.uniform_3_i32(Some(l), v.x, v.y, v.z));
    define_target!(IVec4, |g: &Context, l, v: IVec4| g.uniform_4_i32(Some(l), v.x, v.y, v.z, v.w));
    define_target!(bool, |g: &Context, l, v| g.uniform_1_i32(Some(l), if v { 1 } else { 0 }));
    define_target!(glam::Mat2, |g: &Context, l, v: glam::Mat2| { g.uniform_matrix_2_f32_slice(Some(l), false, v.as_ref().as_slice()) });
    define_target!(glam::Mat3, |g: &Context, l, v: glam::Mat3| { g.uniform_matrix_3_f32_slice(Some(l), false, v.as_ref().as_slice()) });
    define_target!(glam::Mat4, |g: &Context, l, v: glam::Mat4| { g.uniform_matrix_4_f32_slice(Some(l), false, v.as_ref().as_slice()) });
    define_target!(glam::Quat, |g: &Context, l, v: glam::Quat| { g.uniform_4_f32_slice(Some(l), v.as_ref().as_slice()) });
}

use crate::gl::raii::shader::{Shader, ShaderError};

impl Program {
    pub(crate) fn from_ir<E: PassEventTrait>(
        gl: Arc<glow::Context>,
        ir: IRShader,
    ) -> Result<(Self, AssetMemoryUsage), ShaderError> {
        let program = Program::new(gl.clone())?;

        for (source_type, source) in &ir.sources {
            let shader = Shader::new(gl.clone(), *source_type)?;
            let source = String::from_utf8(source.clone())?;
            shader.set_source(source)?;
            shader.compile()?;
            program.attach_shader(shader);
        }

        program.link()?;

        debug!("Allocated shader program ID: {:?}", program.as_inner());
        // TODO: Approximate memory usage
        Ok((program, AssetMemoryUsage::new(size_of::<Program>(), 0)))
    }

    fn new(gl: Arc<glow::Context>) -> Result<Program, ShaderError> {
        unsafe {
            let id = gl
                .create_program()
                .map_err(ShaderError::ProgramCreationError)?;

            Ok(Program { gl, inner: id })
        }
    }

    fn attach_shader(&self, shader: Shader) {
        unsafe {
            self.gl.attach_shader(self.inner, shader.as_inner());
        }
    }

    fn link(&self) -> Result<(), ShaderError> {
        unsafe {
            self.gl.link_program(self.inner);
            if !self.gl.get_program_link_status(self.inner) {
                let log = self.gl.get_program_info_log(self.inner);
                return Err(ShaderError::LinkError { message: log });
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub fn as_inner(&self) -> glow::Program {
        self.inner
    }

    #[inline(always)]
    pub fn bind(gl: &glow::Context, shader: &Self) {
        unsafe {
            gl.use_program(Some(shader.as_inner()));
        }
    }

    #[inline(always)]
    pub fn unbind(gl: &glow::Context) {
        unsafe {
            gl.use_program(None);
        }
    }

    #[inline(always)]
    pub fn set_uniform<T: UniformTarget>(&self, location: &UniformLocation, value: T) {
        T::set_uniform(&self.gl, location, value);
    }

    #[inline(always)]
    pub fn get_uniform_location(&self, name: &str) -> Result<UniformLocation, ShaderError> {
        unsafe {
            self.gl
                .get_uniform_location(self.inner, name)
                .ok_or_else(|| ShaderError::UnknownUniformLocation(name.to_string()))
        }
    }

    #[inline(always)]
    pub fn get_uniform_block_location(&self, name: &str) -> Result<u32, ShaderError> {
        unsafe {
            match self.gl.get_uniform_block_index(self.inner, name) {
                Some(index) => Ok(index),
                None => Err(ShaderError::UnknownUniformLocation(name.to_string())),
            }
        }
    }

    #[inline(always)]
    pub fn set_uniform_block_binding(&self, location: u32, ubo: u32) {
        unsafe {
            self.gl.uniform_block_binding(self.inner, location, ubo);
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        debug!("Dropping shader program ID: {:?}", self.as_inner());
        unsafe {
            self.gl.delete_program(self.inner);
        }
    }
}
