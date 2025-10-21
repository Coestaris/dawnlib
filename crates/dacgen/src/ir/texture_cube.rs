use crate::ir::texture2d::convert_any;
use crate::ir::{normalize_name, PartialIR};
use crate::user::{UserTextureCubeAsset, UserTextureCubeSource};
use crate::UserAssetFile;
use dawn_assets::ir::texture_cube::{IRTextureCube, IRTextureCubeOrder, IRTextureCubeSideData};
use dawn_assets::ir::IRAsset;
use image::{DynamicImage, GenericImage, GenericImageView};
use std::path::Path;

fn cross_to_faces(cross_path: &DynamicImage) -> anyhow::Result<(Vec<DynamicImage>, usize)> {
    let dimensions = cross_path.dimensions();
    let side = dimensions.0 / 4;
    if side * 3 != dimensions.1 {
        return Err(anyhow::anyhow!(
            "Cross texture must be square and have a width of 4 times the height. Got {}x{}",
            dimensions.0,
            dimensions.1
        ));
    }

    let mut faces = Vec::new();
    let positions = [
        (2 * side, side), // Right
        (0, side),        // Left
        (side, 0),        // Top
        (side, 2 * side), // Bottom
        (side, side),     // Front
        (3 * side, side), // Back
    ];

    for pos in positions.iter() {
        let x = pos.0;
        let y = pos.1;

        // Copy the cross-texture
        let mut face = DynamicImage::new_rgba8(side, side);
        for xx in 0..side {
            for yy in 0..side {
                let pixel = cross_path.get_pixel(x + xx, y + yy);
                face.put_pixel(xx, yy, pixel);
            }
        }

        faces.push(face.clone());
    }

    Ok((faces, side as usize))
}

pub fn convert_texture_cube(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserTextureCubeAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    let (faces, size) = match &user.source {
        UserTextureCubeSource::Cross(cross) => {
            let file = cross.as_path(cache_dir, cwd)?;
            let img = image::open(&file)?;
            cross_to_faces(&img)?
        }
        UserTextureCubeSource::Faces { faces, order: _ } => {
            let mut images = vec![];
            for face in faces {
                let file = face.as_path(cache_dir, cwd)?;
                let img = image::open(&file)?;
                images.push(img);
            }

            // Check that images are the same size
            let size = images[0].dimensions();
            for img in &images {
                if img.dimensions() != size {
                    return Err(anyhow::anyhow!(
                        "All faces must be the same size. Expected {:?}, got {:?}",
                        size,
                        img.dimensions()
                    ));
                }
            }
            if size.0 != size.1 {
                return Err(anyhow::anyhow!(
                    "Texture cube faces must be square. Got {}x{}",
                    size.0,
                    size.1
                ));
            }

            (images, size.0 as usize)
        }
    };

    if faces.len() != 6 {
        return Err(anyhow::anyhow!(
            "Expected 6 faces for texture cube, got {}",
            faces.len()
        ));
    }

    let sides = (0..6_i32)
        .map(|i| -> Result<IRTextureCubeSideData, anyhow::Error> {
            let data = convert_any(
                faces[i as usize].clone(),
                size as u32,
                size as u32,
                &user.pixel_format,
            )
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to convert face {} of texture cube '{}': {}",
                    i,
                    file.path.display(),
                    e
                )
            })?;
            Ok(IRTextureCubeSideData { data })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(vec![PartialIR::new_from_id(
        IRAsset::TextureCube(IRTextureCube {
            sides: sides.try_into().unwrap(),
            order: IRTextureCubeOrder::OpenGL,
            size: size as u32,
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
