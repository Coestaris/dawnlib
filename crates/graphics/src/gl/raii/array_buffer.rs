use crate::gl::bindings;
use crate::gl::bindings::types::GLuint;
use log::debug;

pub enum ArrayBufferUsage {
    StaticDraw,
    DynamicDraw,
}

impl ArrayBufferUsage {
    #[inline(always)]
    fn gl_type(&self) -> GLuint {
        match self {
            ArrayBufferUsage::StaticDraw => bindings::STATIC_DRAW,
            ArrayBufferUsage::DynamicDraw => bindings::DYNAMIC_DRAW,
        }
    }
}
pub struct ArrayBufferBinding<'a> {
    array_buffer: &'a mut ArrayBuffer,
}

impl<'a> ArrayBufferBinding<'a> {
    #[inline(always)]
    fn new(array_buffer: &'a mut ArrayBuffer) -> Self {
        debug!("Binding ArrayBuffer ID: {}", array_buffer.id());
        unsafe {
            bindings::BindBuffer(bindings::ARRAY_BUFFER, array_buffer.id());
        }
        Self { array_buffer }
    }

    pub fn feed<T>(&self, data: &[T], usage: ArrayBufferUsage) {
        unsafe {
            bindings::BufferData(
                bindings::ARRAY_BUFFER,
                (data.len() * size_of::<T>()) as isize,
                data.as_ptr() as *const _,
                usage.gl_type(),
            );
        }
    }
}

impl Drop for ArrayBufferBinding<'_> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            bindings::BindBuffer(bindings::ARRAY_BUFFER, 0);
        }
    }
}

pub struct ArrayBuffer {
    id: GLuint,
}

impl ArrayBuffer {
    pub fn new() -> Option<Self> {
        let mut id: GLuint = 0;
        unsafe {
            bindings::GenBuffers(1, &mut id);
            if id == 0 {
                return None;
            }
        }

        debug!("Allocated ArrayBuffer ID: {}", id);
        Some(ArrayBuffer { id })
    }

    pub fn bind(&mut self) -> ArrayBufferBinding<'_> {
        ArrayBufferBinding::new(self)
    }

    #[inline(always)]
    fn id(&self) -> GLuint {
        self.id
    }
}

impl Drop for ArrayBuffer {
    fn drop(&mut self) {
        debug!("Dropping ArrayBuffer ID: {}", self.id);
        unsafe {
            bindings::DeleteBuffers(1, &self.id);
        }
    }
}
