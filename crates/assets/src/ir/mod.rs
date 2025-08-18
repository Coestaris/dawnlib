pub mod audio;
pub mod mesh;
pub mod notes;
pub mod shader;
pub mod texture;
pub mod material;

use crate::ir::audio::IRAudio;
use crate::ir::mesh::IRMesh;
use crate::ir::notes::IRNotes;
use crate::ir::shader::IRShader;
use crate::ir::texture::IRTexture;
use crate::ir::material::IRMaterial;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IRAsset {
    Unknown,
    Shader(IRShader),
    Audio(IRAudio),
    Texture(IRTexture),
    Notes(IRNotes),
    Mesh(IRMesh),
    Material(IRMaterial),
}

impl Default for IRAsset {
    fn default() -> Self {
        IRAsset::Unknown
    }
}
