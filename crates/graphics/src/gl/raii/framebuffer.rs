use crate::gl::bindings;
use crate::gl::bindings::types::{GLint, GLuint};
use crate::gl::raii::renderbuffer::Renderbuffer;
use crate::gl::raii::texture::Texture;
use glam::UVec2;
use log::debug;

#[derive(Debug, Clone, Copy)]
pub enum FramebufferAttachment {
    Color0,
    Color1,
    Color2,
    Color3,
    Color4,
    Color5,
    Color6,
    Color7,
    Color8,
    Depth,
    Stencil,
    DepthStencil,
}

impl FramebufferAttachment {
    fn to_gl(&self) -> GLuint {
        match self {
            FramebufferAttachment::Color0 => bindings::COLOR_ATTACHMENT0,
            FramebufferAttachment::Color1 => bindings::COLOR_ATTACHMENT1,
            FramebufferAttachment::Color2 => bindings::COLOR_ATTACHMENT2,
            FramebufferAttachment::Color3 => bindings::COLOR_ATTACHMENT3,
            FramebufferAttachment::Color4 => bindings::COLOR_ATTACHMENT4,
            FramebufferAttachment::Color5 => bindings::COLOR_ATTACHMENT5,
            FramebufferAttachment::Color6 => bindings::COLOR_ATTACHMENT6,
            FramebufferAttachment::Color7 => bindings::COLOR_ATTACHMENT7,
            FramebufferAttachment::Color8 => bindings::COLOR_ATTACHMENT8,
            FramebufferAttachment::Depth => bindings::DEPTH_ATTACHMENT,
            FramebufferAttachment::Stencil => bindings::STENCIL_ATTACHMENT,
            FramebufferAttachment::DepthStencil => bindings::DEPTH_STENCIL_ATTACHMENT,
        }
    }
}

pub enum BlitFramebufferFilter {
    Nearest,
    Linear,
}

impl BlitFramebufferFilter {
    fn to_gl(&self) -> GLuint {
        match self {
            BlitFramebufferFilter::Nearest => bindings::NEAREST,
            BlitFramebufferFilter::Linear => bindings::LINEAR,
        }
    }
}

pub enum BlitFramebufferMask {
    Color,
    Depth,
    Stencil,
}

impl BlitFramebufferMask {
    fn to_gl(&self) -> GLuint {
        match self {
            BlitFramebufferMask::Color => bindings::COLOR_BUFFER_BIT,
            BlitFramebufferMask::Depth => bindings::DEPTH_BUFFER_BIT,
            BlitFramebufferMask::Stencil => bindings::STENCIL_BUFFER_BIT,
        }
    }
}

pub struct Framebuffer {
    id: GLuint,
}

impl Framebuffer {
    pub fn new() -> Option<Self> {
        let mut id: u32 = 0;
        unsafe {
            bindings::GenFramebuffers(1, &mut id);
            if id == 0 {
                return None;
            }
        }

        debug!("Allocated Framebuffer ID: {}", id);
        Some(Framebuffer { id })
    }

    pub fn bind_read(buffer: &Framebuffer) {
        unsafe {
            bindings::BindFramebuffer(bindings::READ_FRAMEBUFFER, buffer.id());
        }
    }

    pub fn bind_draw(buffer: &Framebuffer) {
        unsafe {
            bindings::BindFramebuffer(bindings::DRAW_FRAMEBUFFER, buffer.id());
        }
    }

    pub fn bind(buffer: &Framebuffer) {
        unsafe {
            bindings::BindFramebuffer(bindings::FRAMEBUFFER, buffer.id());
        }
    }

    pub fn unbind() {
        unsafe {
            bindings::BindFramebuffer(bindings::FRAMEBUFFER, 0);
        }
    }

    pub fn unbind_draw() {
        unsafe {
            bindings::BindFramebuffer(bindings::DRAW_FRAMEBUFFER, 0);
        }
    }

    pub fn unbind_read() {
        unsafe {
            bindings::BindFramebuffer(bindings::READ_FRAMEBUFFER, 0);
        }
    }

    pub fn attach_renderbuffer(
        &self,
        attachment: FramebufferAttachment,
        renderbuffer: &Renderbuffer,
    ) {
        unsafe {
            bindings::FramebufferRenderbuffer(
                bindings::FRAMEBUFFER,
                attachment.to_gl(),
                bindings::RENDERBUFFER,
                renderbuffer.id,
            );
        }
    }

    pub fn attach_texture_2d(
        &self,
        attachment: FramebufferAttachment,
        texture: &Texture,
        mip_level: i32,
    ) {
        unsafe {
            bindings::FramebufferTexture2D(
                bindings::FRAMEBUFFER,
                attachment.to_gl(),
                bindings::TEXTURE_2D,
                texture.id,
                mip_level,
            );
        }
    }

    pub fn draw_buffers(&self, attachments: &[FramebufferAttachment]) {
        let buffers: Vec<GLuint> = attachments.iter().map(|a| a.to_gl()).collect();
        unsafe {
            bindings::DrawBuffers(buffers.len() as i32, buffers.as_ptr());
        }
    }

    pub fn is_complete(&self) -> bool {
        unsafe {
            bindings::CheckFramebufferStatus(bindings::FRAMEBUFFER)
                == bindings::FRAMEBUFFER_COMPLETE
        }
    }

    pub fn blit_to_default(
        framebuffer: &Framebuffer,
        usize: UVec2,
        mask: BlitFramebufferMask,
        filter: BlitFramebufferFilter,
    ) {
        Framebuffer::bind_read(&framebuffer);
        Framebuffer::unbind_draw();
        unsafe {
            bindings::BlitFramebuffer(
                0,
                0,
                usize.x as GLint,
                usize.y as GLint,
                0,
                0,
                usize.x as GLint,
                usize.y as GLint,
                mask.to_gl(),
                filter.to_gl(),
            );
        }
        Framebuffer::unbind();
    }

    #[inline(always)]
    fn id(&self) -> u32 {
        self.id
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        debug!("Dropping Framebuffer ID: {}", self.id);
        unsafe {
            bindings::DeleteFramebuffers(1, &self.id);
        }
    }
}
