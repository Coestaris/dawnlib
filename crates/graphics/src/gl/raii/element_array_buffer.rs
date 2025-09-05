use glow::HasContext;
use log::debug;

pub enum ElementArrayBufferUsage {
    StaticDraw,
    DynamicDraw,
}

impl ElementArrayBufferUsage {
    #[inline(always)]
    fn gl_type(&self) -> u32 {
        match self {
            ElementArrayBufferUsage::StaticDraw => glow::STATIC_DRAW,
            ElementArrayBufferUsage::DynamicDraw => glow::DYNAMIC_DRAW,
        }
    }
}
pub struct ElementArrayBufferBinding<'g, 'a> {
    gl: &'g glow::Context,
    inner: &'a mut ElementArrayBuffer<'g>,
}

impl<'g, 'a> ElementArrayBufferBinding<'g, 'a> {
    #[inline(always)]
    fn new(gl: &'g glow::Context, array_buffer: &'a mut ElementArrayBuffer<'g>) -> Self {
        debug!(
            "Binding ElementArrayBuffer ID: {:?}",
            array_buffer.as_inner()
        );
        unsafe {
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(array_buffer.as_inner()));
        }
        Self {
            gl,
            inner: array_buffer,
        }
    }

    pub fn feed<T>(&self, data: &[T], usage: ElementArrayBufferUsage) {
        unsafe {
            self.gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                std::slice::from_raw_parts(
                    data.as_ptr() as *const u8,
                    data.len() * std::mem::size_of::<T>(),
                ),
                usage.gl_type(),
            );
        }
    }
}

pub struct ElementArrayBuffer<'g> {
    gl: &'g glow::Context,
    inner: glow::Buffer,
}

impl<'g> ElementArrayBuffer<'g> {
    pub fn new(gl: &'g glow::Context) -> Option<Self> {
        unsafe {
            let id = gl.create_buffer().ok()?;

            debug!("Allocated ElementArrayBuffer ID: {:?}", id);
            Some(ElementArrayBuffer { gl, inner: id })
        }
    }

    pub fn bind(&mut self) -> ElementArrayBufferBinding<'g, '_> {
        ElementArrayBufferBinding::new(self.gl, self)
    }

    #[inline(always)]
    fn as_inner(&self) -> glow::Buffer {
        self.inner
    }
}

impl<'g> Drop for ElementArrayBuffer<'g> {
    fn drop(&mut self) {
        debug!("Dropping ElementArrayBuffer ID: {:?}", self.inner);
        unsafe {
            self.gl.delete_buffer(self.inner);
        }
    }
}
