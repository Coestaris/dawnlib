use crate::AssetID;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IRMaterial {
    /// Albedo texture (the amount of light reflected from the surface)
    /// Can be RGBA or RGB.
    pub albedo: AssetID,
    /// Normal map
    /// Always 3 channels.
    pub normal: AssetID,
    /// Metallic-roughness texture
    /// Always 2 channels: R: Metallic, G: Roughness
    pub metallic_roughness: AssetID,
    /// Occlusion texture (the amount of occlusion on the surface)
    /// Always 1 channel.
    pub occlusion: AssetID,
}

impl Default for IRMaterial {
    fn default() -> Self {
        Self {
            albedo: Default::default(),
            metallic_roughness: Default::default(),
            normal: Default::default(),
            occlusion: Default::default(),
        }
    }
}

impl IRMaterial {
    pub fn memory_usage(&self) -> usize {
        let mut sum = size_of::<IRMaterial>();
        sum += self.albedo.memory_usage();
        sum += self.metallic_roughness.memory_usage();
        sum += self.normal.memory_usage();
        sum += self.occlusion.memory_usage();

        sum
    }
}
