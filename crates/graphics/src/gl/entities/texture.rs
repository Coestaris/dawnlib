use crate::gl::bindings;
use crate::gl::bindings::types::{GLenum, GLint, GLsizei, GLuint};
use crate::gl::entities::shader_program::ShaderProgram;
use crate::passes::events::PassEventTrait;
use dawn_assets::ir::texture::{
    IRPixelDataType, IRPixelFormat, IRTexture, IRTextureFilter, IRTextureType, IRTextureWrap,
};
use dawn_assets::{AssetCastable, AssetMemoryUsage};
use log::debug;

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

fn pf_to_format(format: &IRPixelFormat) -> Result<GLenum, String> {
    Ok(match format {
        IRPixelFormat::R8 => bindings::RED,
        IRPixelFormat::R8G8 => bindings::RG,
        IRPixelFormat::R8G8B8 => bindings::RGB,
        IRPixelFormat::R8G8B8A8 => bindings::RGBA,
        IRPixelFormat::R16 => bindings::RED,
        IRPixelFormat::R16G16 => bindings::RG,
        IRPixelFormat::R16G16B16 => bindings::RGB,
        IRPixelFormat::R16G16B16A16 => bindings::RGBA,
        IRPixelFormat::R32G32B32FLOAT => bindings::RGB,
        IRPixelFormat::R32G32B32A32FLOAT => bindings::RGBA,
        _ => return Err("Unsupported pixel format".to_string()),
    })
}

fn pf_to_internal(format: &IRPixelFormat) -> Result<GLenum, String> {
    Ok(match format {
        IRPixelFormat::R8 => bindings::RED,
        IRPixelFormat::R8G8 => bindings::RG,
        IRPixelFormat::R8G8B8 => bindings::RGB,
        IRPixelFormat::R8G8B8A8 => bindings::RGBA,
        IRPixelFormat::R16 => bindings::RED,
        IRPixelFormat::R16G16 => bindings::RG,
        IRPixelFormat::R16G16B16 => bindings::RGB,
        IRPixelFormat::R16G16B16A16 => bindings::RGBA,
        IRPixelFormat::R32G32B32FLOAT => bindings::RGB,
        IRPixelFormat::R32G32B32A32FLOAT => bindings::RGBA,
        _ => return Err("Unsupported pixel format".to_string()),
    })
}

fn pixel_format_to_gl_type(format: &IRPixelFormat) -> Result<GLenum, String> {
    Ok(match format {
        IRPixelFormat::R8
        | IRPixelFormat::R8G8
        | IRPixelFormat::R8G8B8
        | IRPixelFormat::R8G8B8A8 => bindings::UNSIGNED_BYTE,
        IRPixelFormat::R16
        | IRPixelFormat::R16G16
        | IRPixelFormat::R16G16B16
        | IRPixelFormat::R16G16B16A16 => bindings::UNSIGNED_SHORT,
        IRPixelFormat::R32G32B32FLOAT | IRPixelFormat::R32G32B32A32FLOAT => bindings::FLOAT,
        _ => return Err("Unsupported pixel format".to_string()),
    })
}

impl Texture {
    pub fn from_ir<E: PassEventTrait>(ir: IRTexture) -> Result<(Self, AssetMemoryUsage), String> {
        let texture = Self::new(ir.texture_type.clone())?;

        texture.bind(0);
        texture.set_wrap_s(ir.wrap_s.clone())?;
        texture.set_wrap_t(ir.wrap_t.clone())?;
        texture.set_wrap_r(ir.wrap_r.clone())?;
        texture.set_min_filter(ir.min_filter.clone())?;
        texture.set_mag_filter(ir.mag_filter.clone())?;
        if ir.use_mipmaps {
            texture.generate_mipmap()?;
        }
        match ir.texture_type {
            IRTextureType::Texture2D { width, height } => {
                texture.texture_image_2d(
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
        Texture::unbind(texture.texture_type, 0);

        Ok((texture, AssetMemoryUsage::new(size_of::<Texture>(), 0)))
    }

    pub fn bind(&self, texture_index: usize) {
        unsafe {
            bindings::ActiveTexture(bindings::TEXTURE0 + texture_index as GLenum);
            bindings::BindTexture(self.texture_type, self.id);
        }
    }

    pub fn unbind(texture_type: GLenum, texture_index: usize) {
        unsafe {
            bindings::ActiveTexture(bindings::TEXTURE0 + texture_index as GLenum);
            bindings::BindTexture(texture_type, 0);
        }
    }

    fn set_param(&self, param: GLenum, value: GLint) -> Result<(), String> {
        unsafe {
            bindings::TexParameteri(self.texture_type, param, value);
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
            bindings::GenerateMipmap(self.texture_type);
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
        let internal = pf_to_internal(&pixel_format)?;
        let format = pf_to_format(&pixel_format)?;
        let data_type = pixel_format_to_gl_type(&pixel_format)?;
        debug!(
            "Uploading texture ID: {} ({}x{}, format: {}, type: {}, level {})",
            self.id, width, height, format, data_type, level
        );
        unsafe {
            bindings::TexImage2D(
                self.texture_type,
                level as GLint,
                internal as GLint,
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
