use glow::HasContext;
use log::debug;

pub enum ArrayBufferUsage {
    StaticDraw,
    DynamicDraw,
}

impl ArrayBufferUsage {
    #[inline(always)]
    fn gl_type(&self) -> u32 {
        match self {
            ArrayBufferUsage::StaticDraw => glow::STATIC_DRAW,
            ArrayBufferUsage::DynamicDraw => glow::DYNAMIC_DRAW,
        }
    }
}
pub struct ArrayBufferBinding<'g, 'a> {
    gl: &'g glow::Context,
    inner: &'a mut ArrayBuffer<'g>,
}

impl<'g, 'a> ArrayBufferBinding<'g, 'a> {
    #[inline(always)]
    fn new(gl: &'g glow::Context, array_buffer: &'a mut ArrayBuffer<'g>) -> Self {
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(array_buffer.as_inner()));
        }

        Self { gl, inner: array_buffer }
    }

    pub fn feed<T>(&self, data: &[T], usage: ArrayBufferUsage) {
        unsafe {
            self.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    data.as_ptr() as *const u8,
                    data.len() * std::mem::size_of::<T>(),
                ),
                usage.gl_type(),
            );
        }
    }
}

impl<'g> Drop for ArrayBufferBinding<'g, '_> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
    }
}

#[derive(Debug)]
pub struct ArrayBuffer<'g> {
    gl: &'g glow::Context,
    inner: glow::Buffer,
}

impl<'g> ArrayBuffer<'g> {
    pub fn new(gl: &'g glow::Context) -> Option<Self> {
        unsafe {
            let id = gl.create_buffer().ok()?;

            debug!("Allocated ArrayBuffer ID: {:?}", id);
            Some(ArrayBuffer { gl, inner: id })
        }
    }

    pub fn bind(&mut self) -> ArrayBufferBinding<'g, '_> {
        ArrayBufferBinding::new(self.gl, self)
    }

    #[inline(always)]
    fn as_inner(&self) -> glow::Buffer {
        self.inner
    }
}

impl<'g> Drop for ArrayBuffer<'g> {
    fn drop(&mut self) {
        debug!("Dropping ArrayBuffer ID: {:?}", self.inner);
        unsafe {
            self.gl.delete_buffer(self.inner);
        }
    }
}
