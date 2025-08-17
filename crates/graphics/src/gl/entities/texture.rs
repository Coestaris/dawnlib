use crate::gl::bindings;
use crate::gl::bindings::types::{GLenum, GLint, GLsizei, GLuint};
use crate::passes::events::PassEventTrait;
use log::debug;
use dawn_assets::AssetCastable;
use dawn_assets::raw::{PixelDataType, PixelFormat, TextureAssetRaw, TextureFilter, TextureType, TextureWrap};

#[derive(Debug)]
pub struct Texture {
    id: GLuint,
    texture_type: GLuint,
}

impl AssetCastable for Texture {}

fn tex_type_to_gl(tex_type: &TextureType) -> Result<GLuint, String> {
    Ok(match tex_type {
        TextureType::Texture2D { .. } => bindings::TEXTURE_2D,
        TextureType::TextureCube { .. } => bindings::TEXTURE_CUBE_MAP,
        _ => return Err("Unsupported texture type".to_string()),
    })
}

fn wrap_to_gl(wrap: &TextureWrap) -> Result<GLenum, String> {
    Ok(match wrap {
        TextureWrap::ClampToEdge => bindings::CLAMP_TO_EDGE,
        TextureWrap::MirroredRepeat => bindings::MIRRORED_REPEAT,
        TextureWrap::Repeat => bindings::REPEAT,
        _ => return Err("Unsupported texture wrap".to_string()),
    })
}

fn filter_to_gl(filter: &TextureFilter) -> Result<GLenum, String> {
    Ok(match filter {
        TextureFilter::Nearest => bindings::NEAREST,
        TextureFilter::Linear => bindings::LINEAR,
        // TextureFilter::NearestMipmapNearest => bindings::NEAREST_MIPMAP_NEAREST,
        // TextureFilter::LinearMipmapNearest => bindings::LINEAR_MIPMAP_NEAREST,
        // TextureFilter::NearestMipmapLinear => bindings::NEAREST_MIPMAP_LINEAR,
        // TextureFilter::LinearMipmapLinear => bindings::LINEAR_MIPMAP_LINEAR,
        _ => return Err("Unsupported texture filter".to_string()),
    })
}

fn pixel_format_to_gl(format: &PixelFormat) -> Result<GLenum, String> {
    Ok(match format {
        PixelFormat::RGBA(_) => bindings::RGBA,
        PixelFormat::RGB(_) => bindings::RGB,
        PixelFormat::BGRA(_) => bindings::BGRA,
        PixelFormat::BGR(_) => bindings::BGR,
        PixelFormat::SRGB(_) => bindings::SRGB,
        PixelFormat::R8 => bindings::R8,
        PixelFormat::R16 => bindings::R16,
        PixelFormat::R32F => bindings::R32F,
        PixelFormat::RG8 => bindings::RG8,
        PixelFormat::RG16 => bindings::RG16,
        PixelFormat::RG32F => bindings::RG32F,
        _ => return Err("Unsupported pixel format".to_string()),
    })
}

fn pixel_format_to_gl_type(format: &PixelFormat) -> Result<GLenum, String> {
    Ok(match format {
        PixelFormat::RGBA(PixelDataType::U8) => bindings::UNSIGNED_BYTE,
        PixelFormat::RGBA(PixelDataType::U16) => bindings::UNSIGNED_SHORT,
        PixelFormat::RGBA(PixelDataType::F32) => bindings::FLOAT,
        PixelFormat::RGB(PixelDataType::U8) => bindings::UNSIGNED_BYTE,
        PixelFormat::RGB(PixelDataType::U16) => bindings::UNSIGNED_SHORT,
        PixelFormat::RGB(PixelDataType::F32) => bindings::FLOAT,
        PixelFormat::BGRA(PixelDataType::U8) => bindings::UNSIGNED_BYTE,
        PixelFormat::BGRA(PixelDataType::U16) => bindings::UNSIGNED_SHORT,
        PixelFormat::BGRA(PixelDataType::F32) => bindings::FLOAT,
        PixelFormat::BGR(PixelDataType::U8) => bindings::UNSIGNED_BYTE,
        PixelFormat::BGR(PixelDataType::U16) => bindings::UNSIGNED_SHORT,
        PixelFormat::BGR(PixelDataType::F32) => bindings::FLOAT,
        PixelFormat::SRGB(PixelDataType::U8) => bindings::UNSIGNED_BYTE,
        PixelFormat::SRGB(PixelDataType::U16) => bindings::UNSIGNED_SHORT,
        PixelFormat::SRGB(PixelDataType::F32) => bindings::FLOAT,
        PixelFormat::R8 => bindings::UNSIGNED_BYTE,
        PixelFormat::R16 => bindings::UNSIGNED_SHORT,
        PixelFormat::R32F => bindings::FLOAT,
        PixelFormat::RG8 => bindings::UNSIGNED_BYTE,
        PixelFormat::RG16 => bindings::UNSIGNED_SHORT,
        PixelFormat::RG32F => bindings::FLOAT,
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

    pub fn set_wrap_s(&self, wrap: TextureWrap) -> Result<(), String> {
        self.set_param(bindings::TEXTURE_WRAP_S, wrap_to_gl(&wrap)? as GLint)
    }

    pub fn set_wrap_t(&self, wrap: TextureWrap) -> Result<(), String> {
        self.set_param(bindings::TEXTURE_WRAP_T, wrap_to_gl(&wrap)? as GLint)
    }

    pub fn set_wrap_r(&self, wrap: TextureWrap) -> Result<(), String> {
        self.set_param(bindings::TEXTURE_WRAP_R, wrap_to_gl(&wrap)? as GLint)
    }

    pub fn set_min_filter(&self, filter: TextureFilter) -> Result<(), String> {
        self.set_param(
            bindings::TEXTURE_MIN_FILTER,
            filter_to_gl(&filter)? as GLint,
        )
    }

    pub fn set_mag_filter(&self, filter: TextureFilter) -> Result<(), String> {
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
        pixel_format: PixelFormat,
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
            bindings::BindTexture(self.texture.texture_type, 0);
        }
    }
}

impl Texture {
    pub(crate) fn from_raw<E: PassEventTrait>(raw: &TextureAssetRaw) -> Result<Self, String> {
        let texture = Self::new(raw.texture_type.clone())?;
        let binding = texture.bind(0);

        binding.set_wrap_s(raw.wrap_s.clone())?;
        binding.set_wrap_t(raw.wrap_t.clone())?;
        binding.set_wrap_r(raw.wrap_r.clone())?;
        binding.set_min_filter(raw.min_filter.clone())?;
        binding.set_mag_filter(raw.mag_filter.clone())?;
        if raw.use_mipmaps {
            binding.generate_mipmap()?;
        }
        match raw.texture_type {
            TextureType::Texture2D { width, height } => {
                binding.texture_image_2d(
                    0,
                    width as usize,
                    height as usize,
                    false,
                    raw.pixel_format.clone(),
                    &raw.data,
                )?;
            }
            _ => {
                return Err("Unsupported texture type for raw texture".to_string());
            }
        }

        drop(binding);
        Ok(texture)
    }

    pub fn bind(&self, texture_index: usize) -> TextureBinding<'_> {
        TextureBinding::new(self, texture_index)
    }

    #[inline(always)]
    fn id(&self) -> GLuint {
        self.id
    }

    fn new(texture_type: TextureType) -> Result<Self, String> {
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
