use dawn_assets::ir::shader::IRShaderSourceKind;
use glow::HasContext;
use log::{debug, error, info};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use thiserror::Error;

pub struct Shader {
    gl: Arc<glow::Context>,
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

impl Shader {
    pub(crate) fn new(
        gl: Arc<glow::Context>,
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

    fn preprocess<'a>(source: &'a str, custom_defines: &HashMap<String, String>) -> Cow<'a, str> {
        static USER_DEFINES_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
            regex::Regex::new(r#"(?m)^\s*#pragma\s+user_defines\s*$"#)
                .expect("Failed to compile regex")
        });

        if custom_defines.is_empty() {
            return Cow::Borrowed(source);
        }

        let replacement: String = custom_defines
            .iter()
            .map(|(key, value)| format!("#define {} {}\n", key, value))
            .collect();
        let result = USER_DEFINES_REGEX.replace_all(source, replacement.as_str());
        result
    }

    pub fn as_inner(&self) -> glow::Shader {
        self.inner
    }

    pub fn set_source(
        &self,
        source: &str,
        custom_defines: &HashMap<String, String>,
    ) -> Result<(), ShaderError> {
        unsafe {
            let preprocessed = Self::preprocess(source, custom_defines);
            self.gl.shader_source(self.inner, &preprocessed);
        }
        Ok(())
    }

    pub fn compile(&self) -> Result<(), ShaderError> {
        unsafe {
            self.gl.compile_shader(self.inner);

            let log = self.gl.get_shader_info_log(self.inner);
            if !self.gl.get_shader_compile_status(self.inner) {
                return Err(ShaderError::CompilationError { message: log });
            }

            // Get the log and print it if it's not empty (warnings, etc.)
            if !log.is_empty() {
                info!("Shader compilation log: {}", log);
            }
        }
        Ok(())
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        debug!("Dropping shader ID: {:?}", self.inner);
        unsafe {
            self.gl.delete_shader(self.inner);
        }
    }
}
