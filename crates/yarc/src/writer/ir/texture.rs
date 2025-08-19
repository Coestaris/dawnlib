use crate::writer::ir::PartialIR;
use crate::writer::user::{UserAssetHeader, UserTextureAsset};
use crate::writer::UserAssetFile;
use dawn_assets::ir::texture::{IRPixelDataType, IRPixelFormat, IRTexture, IRTextureType};
use image::{DynamicImage, Rgba};
use std::path::{Path, PathBuf};

fn repack(
    image: DynamicImage,
    pixel_format: IRPixelFormat,
    texture_type: IRTextureType,
) -> Result<Vec<u8>, String> {
    match texture_type {
        IRTextureType::Texture2D { width, height } => match pixel_format {
            IRPixelFormat::RGBA(IRPixelDataType::U8) => {
                pack_texture2d(image, width, height, |stream, pixel| {
                    stream.push(pixel[0]); // R
                    stream.push(pixel[1]); // G
                    stream.push(pixel[2]); // B
                    stream.push(pixel[3]); // A
                })
            }
            IRPixelFormat::RGB(IRPixelDataType::U8) => {
                pack_texture2d(image, width, height, |stream, pixel| {
                    stream.push(pixel[0]); // R
                    stream.push(pixel[1]); // G
                    stream.push(pixel[2]); // B
                })
            }
            IRPixelFormat::R8 => {
                pack_texture2d(image, width, height, |stream, pixel| {
                    stream.push(pixel[0]); // R
                })
            }
            _ => Err(format!(
                "Unsupported pixel format for user asset: {:?}",
                pixel_format
            )),
        },
        _ => Err(format!(
            "Unsupported texture type for user asset: {:?}",
            texture_type
        )),
    }
}

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
    image: DynamicImage,
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

pub fn convert_texture(
    file: &UserAssetFile,
    user: &UserTextureAsset,
) -> Result<Vec<PartialIR>, String> {
    // Try to find the file in the same directory as the shader
    let parent = asset_path.parent().unwrap();
    let file = PathBuf::from(user.files[0].clone());
    let file = parent.join(file);

    let img = match image::open(&file) {
        Ok(img) => img,
        Err(e) => {
            return Err(format!(
                "Failed to load texture image '{}': {}",
                file.display(),
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

    Ok(IRTexture {
        data: repack(img, user.pixel_format, texture_type)?,
        texture_type: texture_type.clone(),
        pixel_format: user.pixel_format.clone(),
        use_mipmaps: user.use_mipmaps,
        min_filter: user.min_filter.clone(),
        mag_filter: user.mag_filter.clone(),
        wrap_s: user.wrap_s.clone(),
        wrap_t: user.wrap_t.clone(),
        wrap_r: user.wrap_r.clone(),
    })
}
