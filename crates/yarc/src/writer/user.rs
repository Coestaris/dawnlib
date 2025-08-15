use serde::Deserialize;
use yage2_core::assets::raw::{PixelFormat, ShaderSourceType, TextureFilter, TextureType, TextureWrap};
use yage2_core::assets::AssetHeader;

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UserShaderAsset {
    #[serde(default)]
    pub compile_options: Vec<String>,
    pub files: Vec<(ShaderSourceType, String)>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UserTextureAsset {
    pub files: Vec<String>,
    #[serde(default)]
    pub pixel_format: PixelFormat,
    #[serde(default)]
    pub use_mipmaps: bool,
    #[serde(default)]
    pub min_filter: TextureFilter,
    #[serde(default)]
    pub mag_filter: TextureFilter,
    #[serde(default)]
    pub texture_type: TextureType,
    #[serde(default)]
    pub wrap_s: TextureWrap,
    #[serde(default)]
    pub wrap_t: TextureWrap,
    #[serde(default)]
    pub wrap_r: TextureWrap,
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
