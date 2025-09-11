use glow::HasContext;

pub struct UBO {
    gl: &'static glow::Context,
    inner: glow::Buffer,
}

impl UBO {
    pub fn new(gl: &'static glow::Context, pre_alloc: Option<usize>) -> Option<Self> {
        unsafe {
            let id = gl.create_buffer().ok()?;

            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(id));
            if let Some(size) = pre_alloc {
                gl.buffer_data_size(glow::UNIFORM_BUFFER, size as i32, glow::DYNAMIC_DRAW);
            }
            gl.bind_buffer(glow::UNIFORM_BUFFER, None);

            Some(UBO { gl, inner: id })
        }
    }

    pub fn bind(gl: &glow::Context, ubo: &Self) {
        unsafe {
            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(ubo.inner));
        }
    }

    pub fn unbind(gl: &glow::Context) {
        unsafe {
            gl.bind_buffer(glow::UNIFORM_BUFFER, None);
        }
    }

    pub fn as_inner(&self) -> glow::Buffer {
        self.inner
    }

    pub fn feed(&self, data: &[u8]) {
        unsafe {
            self.gl.buffer_data_u8_slice(
                glow::UNIFORM_BUFFER,
                std::slice::from_raw_parts(
                    data.as_ptr() as *const u8,
                    data.len() * std::mem::size_of::<u8>(),
                ),
                glow::DYNAMIC_DRAW,
            );
        }
    }

    pub fn bind_base(&self, index: u32) {
        unsafe {
            self.gl
                .bind_buffer_base(glow::UNIFORM_BUFFER, index, Some(self.inner));
        }
    }

    pub fn bind_range(&self, index: u32, offset: isize, size: isize) {
        unsafe {
            self.gl.bind_buffer_range(
                glow::UNIFORM_BUFFER,
                index,
                Some(self.inner),
                offset as i32,
                size as i32,
            );
        }
    }
}
