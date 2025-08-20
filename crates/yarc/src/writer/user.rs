use dawn_assets::ir::material::{Emissive, NormalMap, Occlusion};
use dawn_assets::ir::shader::IRShaderSourceType;
use dawn_assets::ir::texture::{IRPixelFormat, IRTextureFilter, IRTextureType, IRTextureWrap};
use dawn_assets::{AssetChecksum, AssetHeader, AssetID, AssetType};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserAssetHeader {
    pub asset_type: AssetType,
    #[serde(default)]
    pub dependencies: Vec<AssetID>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub license: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UserShaderAsset {
    #[serde(default)]
    pub compile_options: Vec<String>,
    pub files: Vec<(IRShaderSourceType, String)>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UserTextureAsset {
    pub files: Vec<String>,
    #[serde(default)]
    pub pixel_format: IRPixelFormat,
    #[serde(default)]
    pub use_mipmaps: bool,
    #[serde(default)]
    pub min_filter: IRTextureFilter,
    #[serde(default)]
    pub mag_filter: IRTextureFilter,
    #[serde(default)]
    pub texture_type: IRTextureType,
    #[serde(default)]
    pub wrap_s: IRTextureWrap,
    #[serde(default)]
    pub wrap_t: IRTextureWrap,
    #[serde(default)]
    pub wrap_r: IRTextureWrap,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UserAudioAsset {
    pub sample_rate: u32,
    pub channels: u8,
    pub file: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UserMeshAsset {
    pub file: String,
    pub gen_material: Option<AssetID>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct UserMaterialAsset {
    pub base_color_factor: [f32; 4],
    pub base_color_texture: Option<String>,
    pub metallic_texture: Option<String>,
    #[serde(default)]
    pub metallic_factor: f32,
    pub roughness_texture: Option<String>,
    #[serde(default)]
    pub roughness_factor: f32,
    // pub normal: Option<NormalMap>,
    // pub occlusion: Option<Occlusion>,
    // pub emissive: Emissive,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum UserAssetProperties {
    Shader(UserShaderAsset),
    Texture(UserTextureAsset),
    Audio(UserAudioAsset),
    Material(UserMaterialAsset),
    Mesh(UserMeshAsset),
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct UserAsset {
    pub header: UserAssetHeader,
    pub properties: UserAssetProperties,
}
