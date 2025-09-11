use crate::gl::raii::texture::Texture;
use crate::passes::events::PassEventTrait;
use dawn_assets::ir::material::IRMaterial;
use dawn_assets::{Asset, AssetCastable, AssetID, AssetMemoryUsage, TypedAsset};
use log::debug;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MaterialError {
    #[error("Requested texture for albedo not found: {0}")]
    AlbedoTextureNotFound(AssetID),
    #[error("Requested texture for metallic not found: {0}")]
    MetallicTextureNotFound(AssetID),
    #[error("Requested texture for roughness not found: {0}")]
    RoughnessTextureNotFound(AssetID),
    #[error("Requested texture for normal not found: {0}")]
    NormalTextureNotFound(AssetID),
    #[error("Requested texture for occlusion not found: {0}")]
    OcclusionTextureNotFound(AssetID),
}

pub struct Material {
    pub albedo: TypedAsset<Texture>,
    pub metallic_roughness: TypedAsset<Texture>,
    pub normal: TypedAsset<Texture>,
    pub occlusion: TypedAsset<Texture>,
}

impl AssetCastable for Material {}

impl Material {
    pub(crate) fn from_ir<E: PassEventTrait>(
        ir: IRMaterial,
        deps: HashMap<AssetID, Asset>,
    ) -> Result<(Self, AssetMemoryUsage), MaterialError> {
        debug!("Creating Material from IR: {:?}", ir);

        let albedo = deps
            .get(&ir.albedo)
            .and_then(|asset| Some(TypedAsset::new(asset.clone())))
            .map(|tex| tex.clone())
            .ok_or(MaterialError::AlbedoTextureNotFound(ir.albedo))?;
        let metallic_roughness = deps
            .get(&ir.metallic_roughness)
            .and_then(|asset| Some(TypedAsset::new(asset.clone())))
            .map(|tex| tex.clone())
            .ok_or(MaterialError::MetallicTextureNotFound(
                ir.metallic_roughness,
            ))?;
        let normal = deps
            .get(&ir.normal)
            .and_then(|asset| Some(TypedAsset::new(asset.clone())))
            .map(|tex| tex.clone())
            .ok_or(MaterialError::NormalTextureNotFound(ir.normal))?;
        let occlusion = deps
            .get(&ir.occlusion)
            .and_then(|asset| Some(TypedAsset::new(asset.clone())))
            .map(|tex| tex.clone())
            .ok_or(MaterialError::OcclusionTextureNotFound(ir.occlusion))?;

        Ok((
            Material {
                albedo,
                metallic_roughness,
                normal,
                occlusion,
            },
            AssetMemoryUsage::new(size_of::<Material>(), 0),
        ))
    }
}
