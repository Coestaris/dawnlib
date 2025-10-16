use crate::gl::raii::renderbuffer::Renderbuffer;
use crate::gl::raii::texture::{GLTexture, Texture2D};
use glam::UVec2;
use glow::HasContext;
use log::debug;
use std::sync::Arc;

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
    fn to_gl(&self) -> u32 {
        match self {
            FramebufferAttachment::Color0 => glow::COLOR_ATTACHMENT0,
            FramebufferAttachment::Color1 => glow::COLOR_ATTACHMENT1,
            FramebufferAttachment::Color2 => glow::COLOR_ATTACHMENT2,
            FramebufferAttachment::Color3 => glow::COLOR_ATTACHMENT3,
            FramebufferAttachment::Color4 => glow::COLOR_ATTACHMENT4,
            FramebufferAttachment::Color5 => glow::COLOR_ATTACHMENT5,
            FramebufferAttachment::Color6 => glow::COLOR_ATTACHMENT6,
            FramebufferAttachment::Color7 => glow::COLOR_ATTACHMENT7,
            FramebufferAttachment::Color8 => glow::COLOR_ATTACHMENT8,
            FramebufferAttachment::Depth => glow::DEPTH_ATTACHMENT,
            FramebufferAttachment::Stencil => glow::STENCIL_ATTACHMENT,
            FramebufferAttachment::DepthStencil => glow::DEPTH_STENCIL_ATTACHMENT,
        }
    }
}

pub enum BlitFramebufferFilter {
    Nearest,
    Linear,
}

impl BlitFramebufferFilter {
    fn to_gl(&self) -> u32 {
        match self {
            BlitFramebufferFilter::Nearest => glow::NEAREST,
            BlitFramebufferFilter::Linear => glow::LINEAR,
        }
    }
}

pub enum BlitFramebufferMask {
    Color,
    Depth,
    Stencil,
}

impl BlitFramebufferMask {
    fn to_gl(&self) -> u32 {
        match self {
            BlitFramebufferMask::Color => glow::COLOR_BUFFER_BIT,
            BlitFramebufferMask::Depth => glow::DEPTH_BUFFER_BIT,
            BlitFramebufferMask::Stencil => glow::STENCIL_BUFFER_BIT,
        }
    }
}

pub struct Framebuffer {
    gl: Arc<glow::Context>,
    inner: glow::Framebuffer,
}

impl Framebuffer {
    pub fn new(gl: Arc<glow::Context>) -> Option<Self> {
        unsafe {
            let id = gl.create_framebuffer().ok()?;

            debug!("Allocated Framebuffer ID: {:?}", id);
            Some(Framebuffer { gl, inner: id })
        }
    }

    pub fn bind_read(gl: &glow::Context, buffer: &Framebuffer) {
        unsafe {
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(buffer.as_inner()));
        }
    }

    pub fn bind_draw(gl: &glow::Context, buffer: &Framebuffer) {
        unsafe {
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(buffer.as_inner()));
        }
    }

    pub fn bind(gl: &glow::Context, buffer: &Framebuffer) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(buffer.as_inner()));
        }
    }

    pub fn unbind(gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }

    pub fn unbind_draw(gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
        }
    }

    pub fn unbind_read(gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
        }
    }

    pub fn attach_renderbuffer(
        &self,
        attachment: FramebufferAttachment,
        renderbuffer: &Renderbuffer,
    ) {
        unsafe {
            self.gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                attachment.to_gl(),
                glow::RENDERBUFFER,
                Some(renderbuffer.as_inner()),
            );
        }
    }

    pub fn attach_texture_2d(
        &self,
        attachment: FramebufferAttachment,
        texture: &Texture2D,
        mip_level: i32,
    ) {
        unsafe {
            self.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                attachment.to_gl(),
                glow::TEXTURE_2D,
                Some(texture.as_inner()),
                mip_level,
            );
        }
    }

    pub fn draw_buffers(&self, attachments: &[FramebufferAttachment]) {
        unsafe {
            self.gl
                .draw_buffers(&attachments.iter().map(|a| a.to_gl()).collect::<Vec<_>>());
        }
    }

    pub fn is_complete(&self) -> bool {
        unsafe { self.gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE }
    }

    pub fn blit_to_default(
        gl: &glow::Context,
        framebuffer: &Framebuffer,
        usize: UVec2,
        mask: BlitFramebufferMask,
        filter: BlitFramebufferFilter,
    ) {
        Framebuffer::bind_read(gl, &framebuffer);
        Framebuffer::unbind_draw(gl);
        unsafe {
            gl.blit_framebuffer(
                0,
                0,
                usize.x as i32,
                usize.y as i32,
                0,
                0,
                usize.x as i32,
                usize.y as i32,
                mask.to_gl(),
                filter.to_gl(),
            );
        }
        Framebuffer::unbind(gl);
    }

    #[inline(always)]
    fn as_inner(&self) -> glow::Framebuffer {
        self.inner
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        debug!("Dropping Framebuffer ID: {:?}", self.inner);
        unsafe {
            self.gl.delete_framebuffer(self.inner);
        }
    }
}
