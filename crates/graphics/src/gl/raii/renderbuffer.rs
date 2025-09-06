use glow::HasContext;
use log::debug;

pub struct Renderbuffer {
    gl: &'static glow::Context,
    inner: glow::Renderbuffer,
}

#[derive(Debug, Clone, Copy)]
pub enum RenderBufferStorage {
    DepthComponent16,
    DepthComponent24,
    DepthComponent32,
    DepthComponent32F,
    Depth24Stencil8,
    Depth32FStencil8,
    R8,
    R16,
    R16F,
    R32F,
}

impl RenderBufferStorage {
    fn to_gl(&self) -> u32 {
        match self {
            RenderBufferStorage::DepthComponent16 => glow::DEPTH_COMPONENT16,
            RenderBufferStorage::DepthComponent24 => glow::DEPTH_COMPONENT24,
            RenderBufferStorage::DepthComponent32 => glow::DEPTH_COMPONENT32,
            RenderBufferStorage::DepthComponent32F => glow::DEPTH_COMPONENT32F,
            RenderBufferStorage::Depth24Stencil8 => glow::DEPTH24_STENCIL8,
            RenderBufferStorage::Depth32FStencil8 => glow::DEPTH32F_STENCIL8,
            RenderBufferStorage::R8 => glow::R8,
            RenderBufferStorage::R16 => glow::R16,
            RenderBufferStorage::R16F => glow::R16F,
            RenderBufferStorage::R32F => glow::R32F,
        }
    }
}

impl Renderbuffer {
    pub fn new(gl: &'static glow::Context) -> Option<Self> {
        unsafe {
            let id = gl.create_renderbuffer().ok()?;

            debug!("Allocated RenderBuffer ID: {:?}", id);
            Some(Renderbuffer { gl, inner: id })
        }
    }

    pub fn bind(gl: &glow::Context, render_buffer: &Renderbuffer) {
        unsafe {
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(render_buffer.as_inner()));
        }
    }

    pub fn unbind(gl: &glow::Context) {
        unsafe {
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
        }
    }

    pub fn as_inner(&self) -> glow::Renderbuffer {
        self.inner
    }

    pub fn storage(&self, storage: RenderBufferStorage, width: usize, height: usize) {
        unsafe {
            self.gl.renderbuffer_storage(
                glow::RENDERBUFFER,
                storage.to_gl(),
                width as i32,
                height as i32,
            );
        }
    }
}

impl Drop for Renderbuffer {
    fn drop(&mut self) {
        debug!("Dropping RenderBuffer ID: {:?}", self.inner);
        unsafe {
            self.gl.delete_renderbuffer(self.inner);
        }
    }
}
