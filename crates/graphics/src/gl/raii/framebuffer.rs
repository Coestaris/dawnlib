use crate::gl::bindings;
use crate::gl::bindings::types::GLuint;
use crate::gl::raii::texture::Texture;
use log::debug;

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
    
    pub fn attach_texture_2d(
        &self,
        attachment: FramebufferAttachment,
        texture: &Texture,
        mip_level: i32,
    ) {
        let gl_attachment = match attachment {
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
        };
        unsafe {
            bindings::FramebufferTexture2D(
                bindings::FRAMEBUFFER,
                gl_attachment,
                bindings::TEXTURE_2D,
                texture.id,
                mip_level,
            );
        }
    }

    pub fn is_complete(&self) -> bool {
        unsafe {
            bindings::CheckFramebufferStatus(bindings::FRAMEBUFFER)
                == bindings::FRAMEBUFFER_COMPLETE
        }
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
