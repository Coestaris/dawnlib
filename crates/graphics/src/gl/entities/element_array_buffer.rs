use crate::gl::bindings;
use crate::gl::bindings::types::GLuint;
use log::debug;

pub enum ElementArrayBufferUsage {
    StaticDraw,
    DynamicDraw,
}

impl ElementArrayBufferUsage {
    #[inline(always)]
    fn gl_type(&self) -> GLuint {
        match self {
            ElementArrayBufferUsage::StaticDraw => bindings::STATIC_DRAW,
            ElementArrayBufferUsage::DynamicDraw => bindings::DYNAMIC_DRAW,
        }
    }
}
pub struct ElementArrayBufferBinding<'a> {
    array_buffer: &'a mut ElementArrayBuffer,
}

impl<'a> ElementArrayBufferBinding<'a> {
    #[inline(always)]
    fn new(array_buffer: &'a mut ElementArrayBuffer) -> Self {
        debug!("Binding ElementArrayBuffer ID: {}", array_buffer.id());
        unsafe {
            bindings::BindBuffer(bindings::ELEMENT_ARRAY_BUFFER, array_buffer.id());
        }
        Self { array_buffer }
    }

    pub fn feed<T>(&self, data: &[T], usage: ElementArrayBufferUsage) -> Result<(), String> {
        unsafe {
            bindings::BufferData(
                bindings::ELEMENT_ARRAY_BUFFER,
                (data.len() * size_of::<T>()) as isize,
                data.as_ptr() as *const _,
                usage.gl_type(),
            );
        }

        Ok(())
    }
}

pub struct ElementArrayBuffer {
    id: GLuint,
}

impl ElementArrayBuffer {
    pub fn new() -> Result<Self, String> {
        let mut id: GLuint = 0;
        unsafe {
            bindings::GenBuffers(1, &mut id);
            if id == 0 {
                return Err("Failed to create VAO".to_string());
            }
        }

        debug!("Allocated ElementArrayBuffer ID: {}", id);
        Ok(ElementArrayBuffer { id })
    }

    pub fn bind(&mut self) -> ElementArrayBufferBinding<'_> {
        ElementArrayBufferBinding::new(self)
    }

    #[inline(always)]
    fn id(&self) -> GLuint {
        self.id
    }
}

impl Drop for ElementArrayBuffer {
    fn drop(&mut self) {
        debug!("Dropping ElementArrayBuffer ID: {}", self.id);
        unsafe {
            bindings::DeleteBuffers(1, &self.id);
        }
    }
}
