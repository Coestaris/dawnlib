use crate::gl::bindings;
use crate::gl::bindings::types::GLuint;
use log::{debug};
use dawn_assets::ir::shader::IRShaderSourceKind;

// RAII wrapper for OpenGL shader
pub struct Shader {
    id: GLuint,
}

impl Shader {
    pub(crate) fn new(source_type: IRShaderSourceKind) -> Result<Shader, String> {
        let gl_type = match source_type {
            IRShaderSourceKind::Vertex => bindings::VERTEX_SHADER,
            IRShaderSourceKind::Fragment => bindings::FRAGMENT_SHADER,
            IRShaderSourceKind::Geometry => bindings::GEOMETRY_SHADER,
            IRShaderSourceKind::Compute => bindings::COMPUTE_SHADER,
            IRShaderSourceKind::TessellationControl => bindings::TESS_CONTROL_SHADER,
            _ => {
                return Err("Unknown shader type".to_string());
            }
        };

        let id = unsafe { bindings::CreateShader(gl_type) };
        if id == 0 {
            return Err("Failed to create shader".to_string());
        }

        debug!("Allocated shader ID: {}", id);
        Ok(Shader { id })
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

impl Drop for Shader {
    fn drop(&mut self) {
        debug!("Dropping shader ID: {}", self.id);
        unsafe {
            bindings::DeleteShader(self.id);
        }
    }
}
