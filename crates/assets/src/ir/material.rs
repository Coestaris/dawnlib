use crate::AssetID;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IRMaterial {
    pub diffuse: AssetID,
    pub specular: AssetID,
    pub normal: AssetID,
    pub height: AssetID,
    pub emissive: AssetID,
}

impl Default for IRMaterial {
    fn default() -> Self {
        Self {
            diffuse: AssetID::default(),
            specular: AssetID::default(),
            normal: AssetID::default(),
            height: AssetID::default(),
            emissive: AssetID::default(),
        }
    }
}
