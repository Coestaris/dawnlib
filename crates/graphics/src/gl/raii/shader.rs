use dawn_assets::ir::shader::IRShaderSourceKind;
use glow::HasContext;
use log::{debug, error};
use thiserror::Error;

pub struct Shader<'g> {
    gl: &'g glow::Context,
    inner: glow::Shader,
}

#[derive(Debug, Error)]
pub enum ShaderError {
    #[error("Failed to create shader")]
    CreationError(String),
    #[error("Failed to create shader program")]
    ProgramCreationError(String),
    #[error("Shader compilation error: {message}")]
    CompilationError { message: String },
    #[error("Failed to link shader program: {message}")]
    LinkError { message: String },
    #[error("Failed to create CString for shader source: {0}")]
    EncodingError(#[from] std::ffi::NulError),
    #[error("Failed to convert shader source to UTF-8: {0}")]
    UTFError(#[from] std::string::FromUtf8Error),
    #[error("Unknown uniform location: {0}")]
    UnknownUniformLocation(String),
}

impl<'g> Shader<'g> {
    pub(crate) fn new(
        gl: &glow::Context,
        source_type: IRShaderSourceKind,
    ) -> Result<Shader, ShaderError> {
        unsafe {
            let id = gl
                .create_shader(match source_type {
                    IRShaderSourceKind::Vertex => glow::VERTEX_SHADER,
                    IRShaderSourceKind::Fragment => glow::FRAGMENT_SHADER,
                    IRShaderSourceKind::Geometry => glow::GEOMETRY_SHADER,
                    IRShaderSourceKind::Compute => glow::COMPUTE_SHADER,
                    IRShaderSourceKind::TessellationControl => glow::TESS_CONTROL_SHADER,
                })
                .map_err(ShaderError::CreationError)?;

            debug!("Allocated shader ID: {:?}", id);
            Ok(Shader { gl, inner: id })
        }
    }

    pub fn as_inner(&self) -> glow::Shader {
        self.inner
    }

    pub fn set_source(&self, source: String) -> Result<(), ShaderError> {
        unsafe {
            self.gl.shader_source(self.inner, &source);
        }
        Ok(())
    }

    pub fn compile(&self) -> Result<(), ShaderError> {
        unsafe {
            self.gl.compile_shader(self.inner);

            if !self.gl.get_shader_compile_status(self.inner) {
                return Err(ShaderError::CompilationError {
                    message: self.gl.get_shader_info_log(self.inner),
                });
            }
        }
        Ok(())
    }
}

impl<'g> Drop for Shader<'g> {
    fn drop(&mut self) {
        debug!("Dropping shader ID: {:?}", self.inner);
        unsafe {
            self.gl.delete_shader(self.inner);
        }
    }
}
