use serde::Deserialize;
use yage2_core::assets::raw::{PixelFormat, ShaderSourceType, TextureType};
use yage2_core::assets::AssetHeader;

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UserShaderAsset {
    #[serde(default)]
    pub compile_options: Vec<String>,
    pub files: Vec<(ShaderSourceType, String)>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UserTextureAsset {
    pub texture_type: TextureType,
    pub pixel_format: PixelFormat,
    pub files: Vec<String>,
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
