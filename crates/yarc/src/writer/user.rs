use dawn_assets::AssetHeader;
use serde::Deserialize;
use dawn_assets::ir::shader::IRShaderSourceType;
use dawn_assets::ir::texture::{IRPixelFormat, IRTextureFilter, IRTextureType, IRTextureWrap};

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
pub enum UserAssetProperties {
    Shader(UserShaderAsset),
    Texture(UserTextureAsset),
    Audio(UserAudioAsset),
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UserAsset {
    pub header: AssetHeader,
    pub properties: UserAssetProperties,
}
