use crate::writer::ir::texture::{
    convert_texture_from_memory, pixel_format_of_dynamic_image, texture_type_of_dynamic_image,
    UserTextureAssetInner,
};
use crate::writer::ir::{normalize_name, PartialIR};
use crate::writer::user::{UserAssetHeader, UserMaterialAsset};
use crate::writer::UserAssetFile;
use dawn_assets::ir::material::{Emissive, IRMaterial};
use dawn_assets::ir::texture::{IRPixelFormat, IRTextureFilter, IRTextureWrap};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetID, AssetType};
use image::{DynamicImage, GrayImage, RgbaImage};
use log::debug;
use std::sync::Arc;

pub(crate) struct UserMaterialAssetInner {
    pub base_color_factor: [f32; 4],
    pub base_color_texture: Option<Arc<RgbaImage>>,
    pub metallic_texture: Option<Arc<GrayImage>>,
    pub metallic_factor: f32,
    pub roughness_texture: Option<Arc<GrayImage>>,
    pub roughness_factor: f32,
    // pub normal: Option<NormalMap>,
    // pub occlusion: Option<Occlusion>,
    // pub emissive: Emissive,
}

fn convert_texture(id: AssetID, image: DynamicImage) -> Result<Vec<PartialIR>, String> {
    let header = UserAssetHeader {
        dependencies: vec![],
        tags: vec![],
        author: Some("Auto-generated".to_string()),
        asset_type: AssetType::Texture,
        license: None,
    };

    let pixel_format = pixel_format_of_dynamic_image(&image)?;
    let texture_type = texture_type_of_dynamic_image(&image)?;
    convert_texture_from_memory(
        id,
        header,
        UserTextureAssetInner {
            data: &image,
            pixel_format,
            use_mipmaps: false,
            min_filter: IRTextureFilter::default(),
            mag_filter: IRTextureFilter::default(),
            texture_type,
            wrap_s: IRTextureWrap::default(),
            wrap_t: IRTextureWrap::default(),
            wrap_r: IRTextureWrap::default(),
        },
    )
}

pub fn convert_material_from_memory(
    id: AssetID,
    mut header: UserAssetHeader,
    user: UserMaterialAssetInner,
) -> Result<Vec<PartialIR>, String> {
    let mut result = Vec::new();
    let base_color_texture = if let Some(base_texture) = user.base_color_texture {
        let id = AssetID::new(format!("{}_base_color", id.as_str()));
        header.dependencies.push(id.clone());
        result.extend(convert_texture(
            id.clone(),
            Arc::unwrap_or_clone(base_texture).into(),
        )?);
        Some(id)
    } else {
        None
    };
    let metallic_texture = if let Some(metallic_texture) = user.metallic_texture {
        let id = AssetID::new(format!("{}_metallic", id.as_str()));
        header.dependencies.push(id.clone());
        result.extend(convert_texture(
            id.clone(),
            Arc::unwrap_or_clone(metallic_texture).into(),
        )?);
        Some(id)
    } else {
        None
    };
    let roughness_texture = if let Some(roughness_texture) = user.roughness_texture {
        let id = AssetID::new(format!("{}_roughness", id.as_str()));
        result.extend(convert_texture(
            id.clone(),
            Arc::unwrap_or_clone(roughness_texture).into(),
        )?);
        header.dependencies.push(id.clone());
        Some(id)
    } else {
        None
    };

    result.push(PartialIR::new_from_id(
        IRAsset::Material(IRMaterial {
            base_color_factor: user.base_color_factor,
            base_color_texture,
            metallic_texture,
            metallic_factor: user.metallic_factor,
            roughness_texture,
            roughness_factor: user.roughness_factor,
            normal: None,
            occlusion: None,
            emissive: Emissive {
                texture: None,
                factor: 0.0,
            },
        }),
        header.clone(),
        id,
    ));
    Ok(result)
}

pub fn convert_material(
    file: &UserAssetFile,
    user: &UserMaterialAsset,
) -> Result<Vec<PartialIR>, String> {
    debug!("Converting material: {:?}", file);

    // TODO: Read iamges from disk
    convert_material_from_memory(
        normalize_name(file.path.clone()),
        file.asset.header.clone(),
        UserMaterialAssetInner {
            base_color_factor: user.base_color_factor.clone(),
            base_color_texture: None,
            metallic_texture: None,
            metallic_factor: 0.0,
            roughness_texture: None,
            roughness_factor: 0.0,
        },
    )
}
