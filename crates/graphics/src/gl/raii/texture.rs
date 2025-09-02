use crate::gl::bindings;
use crate::gl::bindings::types::{GLenum, GLint, GLsizei, GLuint};
use crate::passes::events::PassEventTrait;
use dawn_assets::ir::texture::{
    IRPixelFormat, IRTexture, IRTextureFilter, IRTextureType, IRTextureWrap,
};
use dawn_assets::{AssetCastable, AssetMemoryUsage};
use log::debug;
use thiserror::Error;

#[derive(Debug)]
pub struct Texture {
    pub(crate) id: GLuint,
    texture_type: GLuint,
}

#[derive(Debug, Error)]
pub enum TextureError {
    #[error("Failed to create texture")]
    FailedToCreateTexture,
    #[error("Unsupported texture type: {0:?}")]
    UnsupportedTextureType(IRTextureType),
    #[error("Unsupported texture wrap: {0:?}")]
    UnsupportedTextureWrap(IRTextureWrap),
    #[error("Unsupported texture filter: {0:?}")]
    UnsupportedTextureFilter(IRTextureFilter),
    #[error("Unsupported pixel format: {0:?}")]
    UnsupportedPixelFormat(IRPixelFormat),
    #[error("Unsupported pixel format: {0:?}")]
    UnsupportedPixelType(IRPixelFormat),
}

impl AssetCastable for Texture {}

fn tex_type_to_gl(tex_type: &IRTextureType) -> Result<GLuint, TextureError> {
    Ok(match tex_type {
        IRTextureType::Texture2D { .. } => bindings::TEXTURE_2D,
        IRTextureType::TextureCube { .. } => bindings::TEXTURE_CUBE_MAP,
        _ => return Err(TextureError::UnsupportedTextureType(tex_type.clone())),
    })
}

fn wrap_to_gl(wrap: &IRTextureWrap) -> Result<GLenum, TextureError> {
    Ok(match wrap {
        IRTextureWrap::ClampToEdge => bindings::CLAMP_TO_EDGE,
        IRTextureWrap::MirroredRepeat => bindings::MIRRORED_REPEAT,
        IRTextureWrap::Repeat => bindings::REPEAT,
        _ => return Err(TextureError::UnsupportedTextureWrap(wrap.clone())),
    })
}

fn filter_to_gl(filter: &IRTextureFilter) -> Result<GLenum, TextureError> {
    Ok(match filter {
        IRTextureFilter::Nearest => bindings::NEAREST,
        IRTextureFilter::Linear => bindings::LINEAR,
        // TextureFilter::NearestMipmapNearest => bindings::NEAREST_MIPMAP_NEAREST,
        // TextureFilter::LinearMipmapNearest => bindings::LINEAR_MIPMAP_NEAREST,
        // TextureFilter::NearestMipmapLinear => bindings::NEAREST_MIPMAP_LINEAR,
        // TextureFilter::LinearMipmapLinear => bindings::LINEAR_MIPMAP_LINEAR,
        _ => return Err(TextureError::UnsupportedTextureFilter(filter.clone())),
    })
}

struct GLPF {
    pub internal: GLint,
    pub format: GLenum,
    pub data_type: GLenum,
}

impl GLPF {
    fn new(internal: GLenum, format: GLenum, data_type: GLenum) -> Self {
        GLPF {
            internal: internal as GLint,
            format,
            data_type,
        }
    }
}

fn pf_to_gl(format: &IRPixelFormat) -> Result<GLPF, TextureError> {
    Ok(match format {
        IRPixelFormat::R8 => GLPF::new(bindings::R8, bindings::RED, bindings::UNSIGNED_BYTE),
        IRPixelFormat::RG8 => GLPF::new(bindings::RG, bindings::RG, bindings::UNSIGNED_BYTE),
        IRPixelFormat::RGB8 => GLPF::new(bindings::RGB, bindings::RGB, bindings::UNSIGNED_BYTE),
        IRPixelFormat::RGBA8 => GLPF::new(bindings::RGBA, bindings::RGBA, bindings::UNSIGNED_BYTE),
        IRPixelFormat::R16 => GLPF::new(bindings::RED, bindings::RED, bindings::UNSIGNED_SHORT),
        IRPixelFormat::RG16 => GLPF::new(bindings::RG, bindings::RG, bindings::UNSIGNED_SHORT),
        IRPixelFormat::RGB16 => GLPF::new(bindings::RGB, bindings::RGB, bindings::UNSIGNED_SHORT),
        IRPixelFormat::RGBA16 => {
            GLPF::new(bindings::RGBA, bindings::RGBA, bindings::UNSIGNED_SHORT)
        }
        IRPixelFormat::RGB16F => GLPF::new(bindings::RGB16F, bindings::RGB, bindings::FLOAT),
        IRPixelFormat::RGBA16F => GLPF::new(bindings::RGBA16F, bindings::RGBA, bindings::FLOAT),
        IRPixelFormat::RGB32F => GLPF::new(bindings::RGB, bindings::RGB, bindings::FLOAT),
        IRPixelFormat::RGBA32F => GLPF::new(bindings::RGBA, bindings::RGBA, bindings::FLOAT),
        IRPixelFormat::DEPTH32F => GLPF::new(
            bindings::DEPTH_COMPONENT32F,
            bindings::DEPTH_COMPONENT,
            bindings::FLOAT,
        ),
        _ => return Err(TextureError::UnsupportedPixelFormat(format.clone())),
    })
}

impl Texture {
    pub fn from_ir<E: PassEventTrait>(
        ir: IRTexture,
    ) -> Result<(Self, AssetMemoryUsage), TextureError> {
        let texture = Self::new(ir.texture_type.clone())?;

        Texture::bind(texture.texture_type, &texture, 0);
        texture.set_wrap_s(ir.wrap_s.clone())?;
        texture.set_wrap_t(ir.wrap_t.clone())?;
        texture.set_wrap_r(ir.wrap_r.clone())?;
        texture.set_min_filter(ir.min_filter.clone())?;
        texture.set_mag_filter(ir.mag_filter.clone())?;
        if ir.use_mipmaps {
            texture.generate_mipmap();
        }
        match ir.texture_type {
            IRTextureType::Texture2D { width, height } => {
                texture.texture_image_2d(
                    0,
                    width as usize,
                    height as usize,
                    false,
                    ir.pixel_format.clone(),
                    Some(&ir.data),
                )?;
            }
            _ => Err(TextureError::UnsupportedTextureType(
                ir.texture_type.clone(),
            ))?,
        }
        Texture::unbind(texture.texture_type, 0);
        Ok((texture, AssetMemoryUsage::new(size_of::<Texture>(), 0)))
    }

    pub fn bind(texture_type: GLenum, texture: &Self, texture_index: usize) {
        assert!(texture_index < 32);
        assert_eq!(texture_type, texture.texture_type);
        unsafe {
            bindings::ActiveTexture(bindings::TEXTURE0 + texture_index as GLenum);
            bindings::BindTexture(texture_type, texture.id);
        }
    }

    pub fn unbind(texture_type: GLenum, texture_index: usize) {
        assert!(texture_index < 32);
        unsafe {
            bindings::ActiveTexture(bindings::TEXTURE0 + texture_index as GLenum);
            bindings::BindTexture(texture_type, 0);
        }
    }

    fn set_param(&self, param: GLenum, value: GLint) -> Result<(), TextureError> {
        unsafe {
            bindings::TexParameteri(self.texture_type, param, value);
        }
        Ok(())
    }

    pub fn set_wrap_s(&self, wrap: IRTextureWrap) -> Result<(), TextureError> {
        self.set_param(bindings::TEXTURE_WRAP_S, wrap_to_gl(&wrap)? as GLint)
    }

    pub fn set_wrap_t(&self, wrap: IRTextureWrap) -> Result<(), TextureError> {
        self.set_param(bindings::TEXTURE_WRAP_T, wrap_to_gl(&wrap)? as GLint)
    }

    pub fn set_wrap_r(&self, wrap: IRTextureWrap) -> Result<(), TextureError> {
        self.set_param(bindings::TEXTURE_WRAP_R, wrap_to_gl(&wrap)? as GLint)
    }

    pub fn set_min_filter(&self, filter: IRTextureFilter) -> Result<(), TextureError> {
        self.set_param(
            bindings::TEXTURE_MIN_FILTER,
            filter_to_gl(&filter)? as GLint,
        )
    }

    pub fn set_mag_filter(&self, filter: IRTextureFilter) -> Result<(), TextureError> {
        self.set_param(
            bindings::TEXTURE_MAG_FILTER,
            filter_to_gl(&filter)? as GLint,
        )
    }

    pub fn generate_mipmap(&self) {
        unsafe {
            bindings::GenerateMipmap(self.texture_type);
        }
    }

    pub fn texture_image_2d(
        &self,
        level: usize,
        width: usize,
        height: usize,
        border: bool,
        pixel_format: IRPixelFormat,
        data: Option<&[u8]>,
    ) -> Result<(), TextureError> {
        let gl = pf_to_gl(&pixel_format)?;
        unsafe {
            if let IRPixelFormat::R8 = pixel_format {
                bindings::PixelStorei(bindings::UNPACK_ALIGNMENT, 1);
            }
            bindings::TexImage2D(
                self.texture_type,
                level as GLint,
                gl.internal,
                width as GLsizei,
                height as GLsizei,
                if border { 1 } else { 0 } as GLint,
                gl.format,
                gl.data_type,
                if data.is_none() {
                    std::ptr::null()
                } else {
                    data.unwrap().as_ptr() as *const _
                },
            );
            bindings::PixelStorei(bindings::UNPACK_ALIGNMENT, 4);
        }

        Ok(())
    }

    #[inline(always)]
    fn id(&self) -> GLuint {
        self.id
    }

    pub fn new2d() -> Result<Self, TextureError> {
        Self::new(IRTextureType::Texture2D {
            width: 0u32,
            height: 0u32,
        })
    }

    pub fn new(texture_type: IRTextureType) -> Result<Self, TextureError> {
        let mut id: GLuint = 0;
        unsafe {
            bindings::GenTextures(1, &mut id);
            if id == 0 {
                return Err(TextureError::FailedToCreateTexture);
            }
        }

        debug!("Allocated Texture ID: {}", id);
        Ok(Self {
            id,
            texture_type: tex_type_to_gl(&texture_type)?,
        })
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        debug!("Dropping Texture ID: {}", self.id);
        unsafe {
            bindings::DeleteTextures(1, &self.id);
        }
    }
}
