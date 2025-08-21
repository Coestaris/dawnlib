use crate::passes::events::PassEventTrait;
use dawn_assets::ir::material::{Emissive, IRMaterial, NormalMap, Occlusion};
use dawn_assets::{Asset, AssetCastable, AssetID, AssetMemoryUsage};
use glam::Vec4;
use log::debug;
use std::collections::HashMap;

pub struct Material {
    pub base_color_factor: Vec4,
    pub base_color_texture: Option<Asset>,
    pub metallic_texture: Option<Asset>,
    pub metallic_factor: f32,
    pub roughness_texture: Option<Asset>,
    pub roughness_factor: f32,
    // pub normal: Option<NormalMap>,
    // pub occlusion: Option<Occlusion>,
    // pub emissive: Emissive,
}

impl Default for Material {
    fn default() -> Self {
        Material {
            base_color_factor: Vec4::ONE,
            base_color_texture: None,
            metallic_texture: None,
            metallic_factor: 1.0,
            roughness_texture: None,
            roughness_factor: 1.0,
            // normal: None,
            // occlusion: None,
            // emissive: Emissive::default(),
        }
    }
}

impl AssetCastable for Material {}

impl Material {
    pub(crate) fn from_ir<E: PassEventTrait>(
        ir: IRMaterial,
        deps: HashMap<AssetID, Asset>,
    ) -> Result<(Self, AssetMemoryUsage), String> {
        debug!("Creating Material from IR: {:?}", ir);

        let base_color_texture =
            if let Some(texture_id) = ir.base_color_texture {
                Some(deps.get(&texture_id).cloned().ok_or_else(|| {
                    format!("Base color texture with ID {} not found", texture_id)
                })?)
            } else {
                None
            };
        let metallic_texture = if let Some(texture_id) = ir.metallic_texture {
            Some(
                deps.get(&texture_id)
                    .cloned()
                    .ok_or_else(|| format!("Metallic texture with ID {} not found", texture_id))?,
            )
        } else {
            None
        };
        let roughness_texture = if let Some(texture_id) = ir.roughness_texture {
            Some(
                deps.get(&texture_id)
                    .cloned()
                    .ok_or_else(|| format!("Roughness texture with ID {} not found", texture_id))?,
            )
        } else {
            None
        };
        Ok((
            Material {
                base_color_factor: Vec4::from_array(ir.base_color_factor),
                base_color_texture,
                metallic_texture,
                metallic_factor: ir.metallic_factor,
                roughness_texture,
                roughness_factor: ir.roughness_factor,
                // normal: ir.normal.map(|n| NormalMap::from_ir(n, deps.clone())),
                // occlusion: ir.occlusion.map(|o| Occlusion::from_ir(o, deps.clone())),
                // emissive: ir.emissive.map(|e| Emissive::from_ir(e, deps.clone())).unwrap_or_default(),
            },
            AssetMemoryUsage::new(size_of::<Material>(), 0),
        ))
    }
}
