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
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum UserAssetProperties {
    Shader(UserShaderAsset),
    Texture(UserTextureAsset),
    Audio(UserAudioAsset),
    Mesh(UserMeshAsset),
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UserAsset {
    pub header: UserAssetHeader,
    pub properties: UserAssetProperties,
}
