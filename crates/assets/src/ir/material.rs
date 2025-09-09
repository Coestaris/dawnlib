use crate::AssetID;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IRMaterial {
    pub albedo: AssetID,
    pub metallic_roughness: AssetID,
    pub normal: AssetID,
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
