use crate::ir::{normalize_name, PartialIR};
use crate::user::{UserAssetHeader, UserTexture2DAsset, UserTextureCubeAsset};
use crate::UserAssetFile;
use anyhow::anyhow;
use dawn_assets::ir::texture2d::{IRPixelFormat, IRTexture2D, IRTextureWrap};
use dawn_assets::ir::texture_cube::{IRTextureCube, IRTextureCubeOrder, IRTextureCubeSideData};
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetID;
use image::{DynamicImage, Rgba};
use std::path::Path;

pub fn convert_texture_cube(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserTextureCubeAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    // let texture = user.source.as_path(cache_dir, cwd)?;
    // let img = image::open(&texture)?;

    // let width = img.width();
    let width = 1024;
    let data = vec![];

    Ok(vec![PartialIR::new_from_id(
        IRAsset::TextureCube(IRTextureCube {
            sides: [
                IRTextureCubeSideData { data: data.clone() },
                IRTextureCubeSideData { data: data.clone() },
                IRTextureCubeSideData { data: data.clone() },
                IRTextureCubeSideData { data: data.clone() },
                IRTextureCubeSideData { data: data.clone() },
                IRTextureCubeSideData { data: data.clone() },
            ],
            order: IRTextureCubeOrder::OpenGL,
            size: 0,
            pixel_format: user.pixel_format.clone(),
            use_mipmaps: user.use_mipmaps,
            min_filter: user.min_filter.clone(),
            mag_filter: user.mag_filter.clone(),
            wrap_s: user.wrap_s.clone(),
            wrap_t: user.wrap_t.clone(),
            wrap_r: Default::default(),
        }),
        file.asset.header.clone(),
        normalize_name(file.path.to_path_buf()),
    )])
}
