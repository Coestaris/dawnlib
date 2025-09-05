pub mod audio;
pub mod dictionary;
pub mod font;
pub mod material;
pub mod mesh;
pub mod notes;
pub mod shader;
pub mod texture;

use crate::ir::audio::IRAudio;
use crate::ir::dictionary::IRDictionary;
use crate::ir::font::IRFont;
use crate::ir::material::IRMaterial;
use crate::ir::mesh::IRMesh;
use crate::ir::notes::IRNotes;
use crate::ir::shader::IRShader;
use crate::ir::texture::IRTexture;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum IRAsset {
    Unknown,
    Shader(IRShader),
    Audio(IRAudio),
    Texture(IRTexture),
    Notes(IRNotes),
    Mesh(IRMesh),
    Material(IRMaterial),
    Font(IRFont),
    Dictionary(IRDictionary),
}

impl Default for IRAsset {
    fn default() -> Self {
        IRAsset::Unknown
    }
}

impl IRAsset {
    pub fn memory_usage(&self) -> usize {
        match self {
            IRAsset::Unknown => 0,
            IRAsset::Shader(shader) => shader.memory_usage(),
            IRAsset::Audio(audio) => audio.memory_usage(),
            IRAsset::Texture(texture) => texture.memory_usage(),
            IRAsset::Notes(notes) => notes.memory_usage(),
            IRAsset::Mesh(mesh) => mesh.memory_usage(),
            IRAsset::Material(material) => material.memory_usage(),
            IRAsset::Font(font) => font.memory_usage(),
            IRAsset::Dictionary(dictionary) => dictionary.memory_usage(),
        }
    }
}
