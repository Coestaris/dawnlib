use crate::AssetID;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NormalMap {
    pub texture: AssetID,
    pub scale: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Occlusion {
    pub texture: AssetID,
    pub scale: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Emissive {
    pub texture: Option<AssetID>,
    pub factor: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IRMaterial {
    pub base_color_factor: [f32; 4],
    pub base_color_texture: Option<AssetID>,
    pub metallic_texture: Option<AssetID>,
    pub metallic_factor: f32,
    pub roughness_texture: Option<AssetID>,
    pub roughness_factor: f32,
    pub normal: Option<NormalMap>,
    pub occlusion: Option<Occlusion>,
    pub emissive: Emissive,
}

impl Default for IRMaterial {
    fn default() -> Self {
        Self {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            base_color_texture: None,
            metallic_texture: None,
            metallic_factor: 0.0,
            roughness_texture: None,
            roughness_factor: 0.0,
            normal: None,
            occlusion: None,
            emissive: Emissive {
                texture: None,
                factor: 0.0,
            },
        }
    }
}
