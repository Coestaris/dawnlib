use crate::gl::bindings;
use crate::gl::bindings::types::{GLenum, GLint, GLsizei, GLuint};
use crate::passes::events::PassEventTrait;
use log::debug;
use dawn_assets::{AssetCastable, AssetMemoryUsage};
use dawn_assets::ir::texture::{IRPixelDataType, IRPixelFormat, IRTexture, IRTextureFilter, IRTextureType, IRTextureWrap};
use crate::gl::entities::shader_program::ShaderProgram;

#[derive(Debug)]
pub struct Texture {
    id: GLuint,
    texture_type: GLuint,
}

impl AssetCastable for Texture {}

fn tex_type_to_gl(tex_type: &IRTextureType) -> Result<GLuint, String> {
    Ok(match tex_type {
        IRTextureType::Texture2D { .. } => bindings::TEXTURE_2D,
        IRTextureType::TextureCube { .. } => bindings::TEXTURE_CUBE_MAP,
        _ => return Err("Unsupported texture type".to_string()),
    })
}

fn wrap_to_gl(wrap: &IRTextureWrap) -> Result<GLenum, String> {
    Ok(match wrap {
        IRTextureWrap::ClampToEdge => bindings::CLAMP_TO_EDGE,
        IRTextureWrap::MirroredRepeat => bindings::MIRRORED_REPEAT,
        IRTextureWrap::Repeat => bindings::REPEAT,
        _ => return Err("Unsupported texture wrap".to_string()),
    })
}

fn filter_to_gl(filter: &IRTextureFilter) -> Result<GLenum, String> {
    Ok(match filter {
        IRTextureFilter::Nearest => bindings::NEAREST,
        IRTextureFilter::Linear => bindings::LINEAR,
        // TextureFilter::NearestMipmapNearest => bindings::NEAREST_MIPMAP_NEAREST,
        // TextureFilter::LinearMipmapNearest => bindings::LINEAR_MIPMAP_NEAREST,
        // TextureFilter::NearestMipmapLinear => bindings::NEAREST_MIPMAP_LINEAR,
        // TextureFilter::LinearMipmapLinear => bindings::LINEAR_MIPMAP_LINEAR,
        _ => return Err("Unsupported texture filter".to_string()),
    })
}

fn pixel_format_to_gl(format: &IRPixelFormat) -> Result<GLenum, String> {
    Ok(match format {
        IRPixelFormat::RGBA(_) => bindings::RGBA,
        IRPixelFormat::RGB(_) => bindings::RGB,
        IRPixelFormat::BGRA(_) => bindings::BGRA,
        IRPixelFormat::BGR(_) => bindings::BGR,
        IRPixelFormat::SRGB(_) => bindings::SRGB,
        IRPixelFormat::R8 => bindings::R8,
        IRPixelFormat::R16 => bindings::R16,
        IRPixelFormat::R32F => bindings::R32F,
        IRPixelFormat::RG8 => bindings::RG8,
        IRPixelFormat::RG16 => bindings::RG16,
        IRPixelFormat::RG32F => bindings::RG32F,
        _ => return Err("Unsupported pixel format".to_string()),
    })
}

fn pixel_format_to_gl_type(format: &IRPixelFormat) -> Result<GLenum, String> {
    Ok(match format {
        IRPixelFormat::RGBA(IRPixelDataType::U8) => bindings::UNSIGNED_BYTE,
        IRPixelFormat::RGBA(IRPixelDataType::U16) => bindings::UNSIGNED_SHORT,
        IRPixelFormat::RGBA(IRPixelDataType::F32) => bindings::FLOAT,
        IRPixelFormat::RGB(IRPixelDataType::U8) => bindings::UNSIGNED_BYTE,
        IRPixelFormat::RGB(IRPixelDataType::U16) => bindings::UNSIGNED_SHORT,
        IRPixelFormat::RGB(IRPixelDataType::F32) => bindings::FLOAT,
        IRPixelFormat::BGRA(IRPixelDataType::U8) => bindings::UNSIGNED_BYTE,
        IRPixelFormat::BGRA(IRPixelDataType::U16) => bindings::UNSIGNED_SHORT,
        IRPixelFormat::BGRA(IRPixelDataType::F32) => bindings::FLOAT,
        IRPixelFormat::BGR(IRPixelDataType::U8) => bindings::UNSIGNED_BYTE,
        IRPixelFormat::BGR(IRPixelDataType::U16) => bindings::UNSIGNED_SHORT,
        IRPixelFormat::BGR(IRPixelDataType::F32) => bindings::FLOAT,
        IRPixelFormat::SRGB(IRPixelDataType::U8) => bindings::UNSIGNED_BYTE,
        IRPixelFormat::SRGB(IRPixelDataType::U16) => bindings::UNSIGNED_SHORT,
        IRPixelFormat::SRGB(IRPixelDataType::F32) => bindings::FLOAT,
        IRPixelFormat::R8 => bindings::UNSIGNED_BYTE,
        IRPixelFormat::R16 => bindings::UNSIGNED_SHORT,
        IRPixelFormat::R32F => bindings::FLOAT,
        IRPixelFormat::RG8 => bindings::UNSIGNED_BYTE,
        IRPixelFormat::RG16 => bindings::UNSIGNED_SHORT,
        IRPixelFormat::RG32F => bindings::FLOAT,
        _ => return Err("Unsupported pixel format".to_string()),
    })
}

pub struct TextureBinding<'a> {
    texture: &'a Texture,
}

impl<'a> TextureBinding<'a> {
    pub fn new(texture: &'a Texture, index: usize) -> Self {
        assert!(index < 32, "Texture index must be less than 32");
        unsafe {
            bindings::ActiveTexture(bindings::TEXTURE0 + index as GLenum);
            bindings::BindTexture(texture.texture_type, texture.id);
        }
        Self { texture }
    }

    fn set_param(&self, param: GLenum, value: GLint) -> Result<(), String> {
        unsafe {
            bindings::TexParameteri(self.texture.texture_type, param, value);
        }
        Ok(())
    }

    pub fn set_wrap_s(&self, wrap: IRTextureWrap) -> Result<(), String> {
        self.set_param(bindings::TEXTURE_WRAP_S, wrap_to_gl(&wrap)? as GLint)
    }

    pub fn set_wrap_t(&self, wrap: IRTextureWrap) -> Result<(), String> {
        self.set_param(bindings::TEXTURE_WRAP_T, wrap_to_gl(&wrap)? as GLint)
    }

    pub fn set_wrap_r(&self, wrap: IRTextureWrap) -> Result<(), String> {
        self.set_param(bindings::TEXTURE_WRAP_R, wrap_to_gl(&wrap)? as GLint)
    }

    pub fn set_min_filter(&self, filter: IRTextureFilter) -> Result<(), String> {
        self.set_param(
            bindings::TEXTURE_MIN_FILTER,
            filter_to_gl(&filter)? as GLint,
        )
    }

    pub fn set_mag_filter(&self, filter: IRTextureFilter) -> Result<(), String> {
        self.set_param(
            bindings::TEXTURE_MAG_FILTER,
            filter_to_gl(&filter)? as GLint,
        )
    }

    pub fn generate_mipmap(&self) -> Result<(), String> {
        unsafe {
            bindings::GenerateMipmap(self.texture.texture_type);
        }
        Ok(())
    }

    pub fn texture_image_2d(
        &self,
        level: usize,
        width: usize,
        height: usize,
        border: bool,
        pixel_format: IRPixelFormat,
        data: &[u8],
    ) -> Result<(), String> {
        let format = pixel_format_to_gl(&pixel_format)?;
        let data_type = pixel_format_to_gl_type(&pixel_format)?;
        unsafe {
            bindings::TexImage2D(
                self.texture.texture_type,
                level as GLint,
                format as GLint,
                width as GLsizei,
                height as GLsizei,
                if border { 1 } else { 0 } as GLint,
                format,
                data_type,
                data.as_ptr() as *const _,
            );
        }

        Ok(())
    }
}

impl Drop for TextureBinding<'_> {
    fn drop(&mut self) {
        unsafe {
            // bindings::BindTexture(self.texture.texture_type, 0);
        }
    }
}

impl Texture {
    pub(crate) fn from_ir<E: PassEventTrait>(ir: IRTexture) -> Result<(Self, AssetMemoryUsage), String> {
        let texture = Self::new(ir.texture_type.clone())?;
        let binding = texture.bind(0);

        binding.set_wrap_s(ir.wrap_s.clone())?;
        binding.set_wrap_t(ir.wrap_t.clone())?;
        binding.set_wrap_r(ir.wrap_r.clone())?;
        binding.set_min_filter(ir.min_filter.clone())?;
        binding.set_mag_filter(ir.mag_filter.clone())?;
        if ir.use_mipmaps {
            binding.generate_mipmap()?;
        }
        match ir.texture_type {
            IRTextureType::Texture2D { width, height } => {
                binding.texture_image_2d(
                    0,
                    width as usize,
                    height as usize,
                    false,
                    ir.pixel_format.clone(),
                    &ir.data,
                )?;
            }
            _ => {
                return Err("Unsupported texture type for raw texture".to_string());
            }
        }

        drop(binding);
        Ok((texture, AssetMemoryUsage::new(size_of::<Texture>(), 0)))
    }

    pub fn bind(&self, texture_index: usize) -> TextureBinding<'_> {
        TextureBinding::new(self, texture_index)
    }

    #[inline(always)]
    fn id(&self) -> GLuint {
        self.id
    }

    fn new(texture_type: IRTextureType) -> Result<Self, String> {
        let mut id: GLuint = 0;
        unsafe {
            bindings::GenTextures(1, &mut id);
            if id == 0 {
                return Err("Failed to create texture".to_string());
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
