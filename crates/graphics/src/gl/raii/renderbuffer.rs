use crate::gl::bindings;
use crate::gl::bindings::types::GLuint;
use log::debug;

pub struct Renderbuffer {
    pub id: GLuint,
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
    fn to_gl(&self) -> GLuint {
        match self {
            RenderBufferStorage::DepthComponent16 => bindings::DEPTH_COMPONENT16,
            RenderBufferStorage::DepthComponent24 => bindings::DEPTH_COMPONENT24,
            RenderBufferStorage::DepthComponent32 => bindings::DEPTH_COMPONENT32,
            RenderBufferStorage::DepthComponent32F => bindings::DEPTH_COMPONENT32F,
            RenderBufferStorage::Depth24Stencil8 => bindings::DEPTH24_STENCIL8,
            RenderBufferStorage::Depth32FStencil8 => bindings::DEPTH32F_STENCIL8,
            RenderBufferStorage::R8 => bindings::R8,
            RenderBufferStorage::R16 => bindings::R16,
            RenderBufferStorage::R16F => bindings::R16F,
            RenderBufferStorage::R32F => bindings::R32F,
        }
    }
}

impl Renderbuffer {
    pub fn new() -> Option<Self> {
        let mut id: u32 = 0;
        unsafe {
            bindings::GenRenderbuffers(1, &mut id);
            if id == 0 {
                return None;
            }
        }

        debug!("Allocated RenderBuffer ID: {}", id);
        Some(Renderbuffer { id })
    }

    pub fn bind(render_buffer: &Renderbuffer) {
        unsafe {
            bindings::BindRenderbuffer(bindings::RENDERBUFFER, render_buffer.id);
        }
    }

    pub fn unbind() {
        unsafe {
            bindings::BindRenderbuffer(bindings::RENDERBUFFER, 0);
        }
    }

    pub fn storage(&self, storage: RenderBufferStorage, width: usize, height: usize) {
        unsafe {
            bindings::RenderbufferStorage(
                bindings::RENDERBUFFER,
                storage.to_gl(),
                width as i32,
                height as i32,
            );
        }
    }
}

impl Drop for Renderbuffer {
    fn drop(&mut self) {
        debug!("Dropping RenderBuffer ID: {}", self.id);
        unsafe {
            bindings::DeleteRenderbuffers(1, &self.id);
        }
    }
}
