use crate::ir::{normalize_name, PartialIR};
use crate::user::{UserAssetHeader, UserTexture2DAsset};
use crate::UserAssetFile;
use anyhow::anyhow;
use dawn_assets::ir::texture2d::{IRPixelFormat, IRTexture2D, IRTextureWrap};
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetID;
use image::{DynamicImage, Rgba};
use std::path::Path;

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
) -> anyhow::Result<Vec<u8>> {
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

pub fn convert_texture2d(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserTexture2DAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    let texture = user.source.as_path(cache_dir, cwd)?;
    let img = image::open(&texture)?;

    let width = img.width();
    let height = img.height();

    let data = match user.pixel_format {
        IRPixelFormat::RGBA8 => {
            pack_texture2d(&img, width, height, |stream, pixel| {
                stream.push(pixel[0]); // R
                stream.push(pixel[1]); // G
                stream.push(pixel[2]); // B
                stream.push(pixel[3]); // A
            })?
        }
        IRPixelFormat::RGB8 => {
            pack_texture2d(&img, width, height, |stream, pixel| {
                stream.push(pixel[0]); // R
                stream.push(pixel[1]); // G
                stream.push(pixel[2]); // B
            })?
        }
        IRPixelFormat::R8 => {
            pack_texture2d(&img, width, height, |stream, pixel| {
                stream.push(pixel[0]); // R
            })?
        }
        _ => {
            return Err(anyhow!(
                "Unsupported pixel format for user asset: {:?}",
                user.pixel_format
            ));
        }
    };

    Ok(vec![PartialIR::new_from_id(
        IRAsset::Texture2D(IRTexture2D {
            data,
            width,
            height,
            pixel_format: user.pixel_format.clone(),
            use_mipmaps: user.use_mipmaps,
            min_filter: user.min_filter.clone(),
            mag_filter: user.mag_filter.clone(),
            wrap_s: user.wrap_s.clone(),
            wrap_t: user.wrap_t.clone(),
        }),
        file.asset.header.clone(),
        normalize_name(file.path.to_path_buf()),
    )])
}
