use crate::writer::ir::{normalize_name, PartialIR};
use crate::writer::user::{UserAssetHeader, UserTextureAsset};
use crate::writer::UserAssetFile;
use dawn_assets::ir::texture::{
    IRPixelDataType, IRPixelFormat, IRTexture, IRTextureFilter, IRTextureType, IRTextureWrap,
};
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetID;
use image::{ColorType, DynamicImage, Rgba};
use log::debug;
use std::path::PathBuf;

struct Stream {
    data: Vec<u8>,
}

impl Stream {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    fn push<T>(&mut self, value: T) {
        let size = size_of::<T>();
        let bytes = unsafe { std::slice::from_raw_parts(&value as *const T as *const u8, size) };
        self.data.extend_from_slice(bytes);
    }

    fn to_vec(self) -> Vec<u8> {
        self.data
    }
}

fn pack_texture2d(
    image: &DynamicImage,
    width: u32,
    height: u32,
    pack: impl Fn(&mut Stream, &Rgba<u8>) -> (),
) -> Result<Vec<u8>, String> {
    let resized =
        DynamicImage::resize_exact(&image, width, height, image::imageops::FilterType::Nearest);
    let resized = resized.to_rgba8();

    let mut stream = Stream::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let pixel = resized.get_pixel(x, y);
            pack(&mut stream, pixel);
        }
    }

    Ok(stream.to_vec())
}

pub(crate) struct UserTextureAssetInner<'a> {
    pub data: &'a DynamicImage,
    pub pixel_format: IRPixelFormat,
    pub use_mipmaps: bool,
    pub min_filter: IRTextureFilter,
    pub mag_filter: IRTextureFilter,
    pub texture_type: IRTextureType,
    pub wrap_s: IRTextureWrap,
    pub wrap_t: IRTextureWrap,
    pub wrap_r: IRTextureWrap,
}

pub fn convert_texture_from_memory(
    id: AssetID,
    header: UserAssetHeader,
    user: UserTextureAssetInner,
) -> Result<Vec<PartialIR>, String> {
    let data = match user.texture_type {
        IRTextureType::Texture2D { width, height } => match user.pixel_format {
            IRPixelFormat::RGBA(IRPixelDataType::U8) => {
                pack_texture2d(user.data, width, height, |stream, pixel| {
                    stream.push(pixel[0]); // R
                    stream.push(pixel[1]); // G
                    stream.push(pixel[2]); // B
                    stream.push(pixel[3]); // A
                })?
            }
            IRPixelFormat::RGB(IRPixelDataType::U8) => {
                pack_texture2d(user.data, width, height, |stream, pixel| {
                    stream.push(pixel[0]); // R
                    stream.push(pixel[1]); // G
                    stream.push(pixel[2]); // B
                })?
            }
            IRPixelFormat::R8 => {
                pack_texture2d(user.data, width, height, |stream, pixel| {
                    stream.push(pixel[0]); // R
                })?
            }
            _ => Err(format!(
                "Unsupported pixel format for user asset: {:?}",
                user.pixel_format
            ))?,
        },
        _ => Err(format!(
            "Unsupported texture type for user asset: {:?}",
            user.texture_type
        ))?,
    };

    Ok(vec![PartialIR::new_from_id(
        IRAsset::Texture(IRTexture {
            data,
            texture_type: user.texture_type.clone(),
            pixel_format: user.pixel_format.clone(),
            use_mipmaps: user.use_mipmaps,
            min_filter: user.min_filter.clone(),
            mag_filter: user.mag_filter.clone(),
            wrap_s: user.wrap_s.clone(),
            wrap_t: user.wrap_t.clone(),
            wrap_r: user.wrap_r.clone(),
        }),
        header.clone(),
        id,
    )])
}

pub fn convert_texture(
    file: &UserAssetFile,
    user: &UserTextureAsset,
) -> Result<Vec<PartialIR>, String> {
    debug!("Converting texture: {:?}", file);

    // Try to find the file in the same directory as the shader
    let parent = file.path.parent().unwrap();
    let texture = PathBuf::from(user.files[0].clone());
    let texture = parent.join(texture);

    let img = match image::open(&texture) {
        Ok(img) => img,
        Err(e) => {
            return Err(format!(
                "Failed to load texture image '{}': {}",
                texture.display(),
                e
            ))
        }
    };

    let texture_type = match user.texture_type {
        IRTextureType::Unknown => IRTextureType::Texture2D {
            width: img.width(),
            height: img.height(),
        },
        any => any,
    };

    convert_texture_from_memory(
        normalize_name(file.path.clone()),
        file.asset.header.clone(),
        UserTextureAssetInner {
            data: &img,
            pixel_format: user.pixel_format.clone(),
            use_mipmaps: user.use_mipmaps,
            min_filter: user.min_filter.clone(),
            mag_filter: user.mag_filter.clone(),
            texture_type,
            wrap_s: user.wrap_s.clone(),
            wrap_t: user.wrap_t.clone(),
            wrap_r: user.wrap_r.clone(),
        },
    )
}

pub fn texture_type_of_dynamic_image(image: &DynamicImage) -> Result<IRTextureType, String> {
    Ok(IRTextureType::Texture2D {
        width: image.width(),
        height: image.height(),
    })
}

pub fn pixel_format_of_dynamic_image(image: &DynamicImage) -> Result<IRPixelFormat, String> {
    Ok(match image.color() {
        ColorType::L8 => IRPixelFormat::R8,
        ColorType::La8 => IRPixelFormat::RG8,
        ColorType::Rgb8 => IRPixelFormat::RGB(IRPixelDataType::U8),
        ColorType::Rgba8 => IRPixelFormat::RGBA(IRPixelDataType::U8),
        ColorType::L16 => IRPixelFormat::R16,
        ColorType::La16 => IRPixelFormat::RG16,
        ColorType::Rgb16 => IRPixelFormat::RGB(IRPixelDataType::U16),
        ColorType::Rgba16 => IRPixelFormat::RGBA(IRPixelDataType::U16),
        ColorType::Rgb32F => IRPixelFormat::RGB(IRPixelDataType::F32),
        ColorType::Rgba32F => IRPixelFormat::RGBA(IRPixelDataType::F32),
        _ => {
            return Err(format!("Unsupported image color type: {:?}", image.color()));
        }
    })
}
