use crate::passes::events::PassEventTrait;
use dawn_assets::ir::texture::{
    IRPixelFormat, IRTexture, IRTextureFilter, IRTextureType, IRTextureWrap,
};
use dawn_assets::{AssetCastable, AssetMemoryUsage};
use glow::HasContext;
use log::debug;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug)]
pub struct Texture {
    gl: Arc<glow::Context>,
    inner: glow::Texture,
    texture_type: TextureBind,
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

fn tex_type_to_gl(tex_type: &IRTextureType) -> Result<TextureBind, TextureError> {
    Ok(match tex_type {
        IRTextureType::Texture2D { .. } => TextureBind::Texture2D,
        IRTextureType::TextureCube { .. } => TextureBind::TextureCubeMap,
        _ => return Err(TextureError::UnsupportedTextureType(tex_type.clone())),
    })
}

fn wrap_to_gl(wrap: &IRTextureWrap) -> Result<u32, TextureError> {
    Ok(match wrap {
        IRTextureWrap::ClampToEdge => glow::CLAMP_TO_EDGE,
        IRTextureWrap::MirroredRepeat => glow::MIRRORED_REPEAT,
        IRTextureWrap::Repeat => glow::REPEAT,
        _ => return Err(TextureError::UnsupportedTextureWrap(wrap.clone())),
    })
}

fn filter_to_gl(filter: &IRTextureFilter) -> Result<u32, TextureError> {
    Ok(match filter {
        IRTextureFilter::Nearest => glow::NEAREST,
        IRTextureFilter::Linear => glow::LINEAR,
        IRTextureFilter::NearestMipmapNearest => glow::NEAREST_MIPMAP_NEAREST,
        IRTextureFilter::LinearMipmapNearest => glow::LINEAR_MIPMAP_NEAREST,
        IRTextureFilter::NearestMipmapLinear => glow::NEAREST_MIPMAP_LINEAR,
        IRTextureFilter::LinearMipmapLinear => glow::LINEAR_MIPMAP_LINEAR,
    })
}

struct GLPF {
    pub internal: u32,
    pub format: u32,
    pub data_type: u32,
}

impl GLPF {
    fn new(internal: u32, format: u32, data_type: u32) -> Self {
        GLPF {
            internal,
            format,
            data_type,
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureBind {
    Texture2D = glow::TEXTURE_2D,
    TextureCubeMap = glow::TEXTURE_CUBE_MAP,
}

fn pf_to_gl(format: &IRPixelFormat) -> Result<GLPF, TextureError> {
    Ok(match format {
        IRPixelFormat::R8 => GLPF::new(glow::R8, glow::RED, glow::UNSIGNED_BYTE),
        IRPixelFormat::RG8 => GLPF::new(glow::RG, glow::RG, glow::UNSIGNED_BYTE),
        IRPixelFormat::RG8_SNORM => GLPF::new(glow::RG8_SNORM, glow::RG, glow::BYTE),
        IRPixelFormat::RGB8 => GLPF::new(glow::RGB, glow::RGB, glow::UNSIGNED_BYTE),
        IRPixelFormat::RGBA8 => GLPF::new(glow::RGBA, glow::RGBA, glow::UNSIGNED_BYTE),
        IRPixelFormat::R16 => GLPF::new(glow::RED, glow::RED, glow::UNSIGNED_SHORT),
        IRPixelFormat::RG16 => GLPF::new(glow::RG, glow::RG, glow::UNSIGNED_SHORT),
        IRPixelFormat::RGB16 => GLPF::new(glow::RGB, glow::RGB, glow::UNSIGNED_SHORT),
        IRPixelFormat::RGBA16 => GLPF::new(glow::RGBA, glow::RGBA, glow::UNSIGNED_SHORT),
        IRPixelFormat::R16F => GLPF::new(glow::R16F, glow::RED, glow::FLOAT),
        IRPixelFormat::RG16F => GLPF::new(glow::RG16F, glow::RG, glow::FLOAT),
        IRPixelFormat::RGB16F => GLPF::new(glow::RGB16F, glow::RGB, glow::FLOAT),
        IRPixelFormat::RGBA16F => GLPF::new(glow::RGBA16F, glow::RGBA, glow::FLOAT),
        IRPixelFormat::R32F => GLPF::new(glow::R32F, glow::RED, glow::FLOAT),
        IRPixelFormat::RG32F => GLPF::new(glow::RG32F, glow::RG, glow::FLOAT),
        IRPixelFormat::RGB32F => GLPF::new(glow::RGB, glow::RGB, glow::FLOAT),
        IRPixelFormat::RGBA32F => GLPF::new(glow::RGBA, glow::RGBA, glow::FLOAT),
        IRPixelFormat::RGBA32UI => {
            GLPF::new(glow::RGBA32UI, glow::RGBA_INTEGER, glow::UNSIGNED_INT)
        }
        IRPixelFormat::DEPTH32F => {
            GLPF::new(glow::DEPTH_COMPONENT32F, glow::DEPTH_COMPONENT, glow::FLOAT)
        }
        IRPixelFormat::DEPTH16 => GLPF::new(
            glow::DEPTH_COMPONENT16,
            glow::DEPTH_COMPONENT,
            glow::UNSIGNED_SHORT,
        ),
        IRPixelFormat::DEPTH24 => GLPF::new(
            glow::DEPTH_COMPONENT24,
            glow::DEPTH_COMPONENT,
            glow::UNSIGNED_INT,
        ),
        _ => return Err(TextureError::UnsupportedPixelFormat(format.clone())),
    })
}

impl Texture {
    pub fn from_ir<E: PassEventTrait>(
        gl: Arc<glow::Context>,
        ir: IRTexture,
    ) -> Result<(Self, AssetMemoryUsage), TextureError> {
        let texture = Self::new(gl.clone(), ir.texture_type.clone())?;

        Texture::bind(&gl, texture.texture_type, &texture, 0);
        texture.set_wrap_s(ir.wrap_s.clone())?;
        texture.set_wrap_t(ir.wrap_t.clone())?;
        texture.set_wrap_r(ir.wrap_r.clone())?;
        texture.set_min_filter(ir.min_filter.clone())?;
        texture.set_mag_filter(ir.mag_filter.clone())?;

        match ir.texture_type {
            IRTextureType::Texture2D { width, height } => {
                texture.feed_2d(
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

        if ir.use_mipmaps {
            texture.generate_mipmap();
        }

        Texture::unbind(&gl, texture.texture_type, 0);
        Ok((
            texture,
            AssetMemoryUsage::new(size_of::<Texture>(), ir.data.len()),
        ))
    }

    pub fn bind(gl: &glow::Context, texture_type: TextureBind, texture: &Self, texture_index: u32) {
        assert!(texture_index < 32);
        assert_eq!(texture_type, texture.texture_type);
        unsafe {
            gl.active_texture(glow::TEXTURE0 + texture_index as u32);
            gl.bind_texture(texture_type as u32, Some(texture.as_inner()));
        }
    }

    pub fn unbind(gl: &glow::Context, texture_type: TextureBind, texture_index: u32) {
        assert!(texture_index < 32);
        unsafe {
            gl.active_texture(glow::TEXTURE0 + texture_index as u32);
            gl.bind_texture(texture_type as u32, None);
        }
    }

    fn set_param(&self, param: u32, value: u32) -> Result<(), TextureError> {
        unsafe {
            self.gl
                .tex_parameter_i32(self.texture_type as u32, param, value as i32);
        }
        Ok(())
    }

    pub fn set_wrap_s(&self, wrap: IRTextureWrap) -> Result<(), TextureError> {
        self.set_param(glow::TEXTURE_WRAP_S, wrap_to_gl(&wrap)?)
    }

    pub fn disable_compare_mode(&self) -> Result<(), TextureError> {
        self.set_param(glow::TEXTURE_COMPARE_MODE, glow::NONE)
    }

    pub fn set_wrap_t(&self, wrap: IRTextureWrap) -> Result<(), TextureError> {
        self.set_param(glow::TEXTURE_WRAP_T, wrap_to_gl(&wrap)?)
    }

    pub fn set_wrap_r(&self, wrap: IRTextureWrap) -> Result<(), TextureError> {
        self.set_param(glow::TEXTURE_WRAP_R, wrap_to_gl(&wrap)?)
    }

    pub fn set_min_filter(&self, filter: IRTextureFilter) -> Result<(), TextureError> {
        self.set_param(glow::TEXTURE_MIN_FILTER, filter_to_gl(&filter)?)
    }

    pub fn set_mag_filter(&self, filter: IRTextureFilter) -> Result<(), TextureError> {
        self.set_param(glow::TEXTURE_MAG_FILTER, filter_to_gl(&filter)?)
    }

    pub fn set_max_level(&self, level: i32) -> Result<(), TextureError> {
        self.set_param(glow::TEXTURE_MAX_LEVEL, level as u32)
    }

    pub fn generate_mipmap(&self) {
        unsafe {
            self.gl.generate_mipmap(self.texture_type as u32);
        }
    }

    pub fn feed_2d<T>(
        &self,
        level: usize,
        width: usize,
        height: usize,
        border: bool,
        pixel_format: IRPixelFormat,
        data: Option<&[T]>,
    ) -> Result<(), TextureError> {
        let gl = pf_to_gl(&pixel_format)?;
        unsafe {
            if let IRPixelFormat::R8 = pixel_format {
                self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            }

            self.gl.tex_image_2d(
                self.texture_type as u32,
                level as i32,
                gl.internal as i32,
                width as i32,
                height as i32,
                if border { 1 } else { 0 },
                gl.format,
                gl.data_type,
                match data {
                    None => glow::PixelUnpackData::Slice(None),
                    Some(d) => glow::PixelUnpackData::Slice(Some(std::slice::from_raw_parts(
                        d.as_ptr() as *const u8,
                        size_of::<T>() * d.len(),
                    ))),
                },
            );
            self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
        }

        Ok(())
    }

    #[inline(always)]
    pub fn as_inner(&self) -> glow::Texture {
        self.inner
    }

    pub fn new2d(gl: Arc<glow::Context>) -> Result<Self, TextureError> {
        Self::new(
            gl,
            IRTextureType::Texture2D {
                width: 0u32,
                height: 0u32,
            },
        )
    }

    pub fn new(gl: Arc<glow::Context>, texture_type: IRTextureType) -> Result<Self, TextureError> {
        unsafe {
            let id = gl
                .create_texture()
                .map_err(|_| TextureError::FailedToCreateTexture)?;

            debug!("Allocated Texture ID: {:?}", id);
            Ok(Texture {
                gl,
                inner: id,
                texture_type: tex_type_to_gl(&texture_type)?,
            })
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        debug!("Dropping Texture ID: {:?}", self.inner);
        unsafe {
            self.gl.delete_texture(self.inner);
        }
    }
}
