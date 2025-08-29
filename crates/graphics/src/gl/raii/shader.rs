use crate::gl::bindings;
use crate::gl::bindings::types::GLuint;
use dawn_assets::ir::shader::IRShaderSourceKind;
use log::{debug, error};
use thiserror::Error;

// RAII wrapper for OpenGL shader
pub struct Shader {
    id: GLuint,
}

#[derive(Debug, Error)]
pub enum ShaderError {
    #[error("Failed to create shader")]
    CreationError,
    #[error("Failed to create shader program")]
    ProgramCreationError,
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

impl Shader {
    pub(crate) fn new(source_type: IRShaderSourceKind) -> Result<Shader, ShaderError> {
        let id = unsafe {
            bindings::CreateShader(match source_type {
                IRShaderSourceKind::Vertex => bindings::VERTEX_SHADER,
                IRShaderSourceKind::Fragment => bindings::FRAGMENT_SHADER,
                IRShaderSourceKind::Geometry => bindings::GEOMETRY_SHADER,
                IRShaderSourceKind::Compute => bindings::COMPUTE_SHADER,
                IRShaderSourceKind::TessellationControl => bindings::TESS_CONTROL_SHADER,
            })
        };
        if id == 0 {
            return Err(ShaderError::CreationError);
        }

        debug!("Allocated shader ID: {}", id);
        Ok(Shader { id })
    }

    pub fn id(&self) -> GLuint {
        self.id
    }

    pub fn set_source(&self, source: String) -> Result<(), ShaderError> {
        let c_source = std::ffi::CString::new(source)?;
        unsafe {
            bindings::ShaderSource(self.id, 1, &c_source.as_ptr(), std::ptr::null());
        }
        Ok(())
    }

    pub fn compile(&self) -> Result<(), ShaderError> {
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
                return Err(ShaderError::CompilationError {
                    message: String::from_utf8(log).unwrap(),
                });
            }
        }
        Ok(())
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        debug!("Dropping shader ID: {}", self.id);
        unsafe {
            bindings::DeleteShader(self.id);
        }
    }
}
