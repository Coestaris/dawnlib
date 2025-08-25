use crate::deep_hash::{with_std, DeepHash, DeepHashCtx};
use crate::source::SourceRef;
use dawn_assets::ir::shader::IRShaderSourceKind;
use dawn_assets::ir::texture::{IRPixelFormat, IRTextureFilter, IRTextureType, IRTextureWrap};
use dawn_assets::{AssetID, AssetType};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct UserAssetHeader {
    pub asset_type: AssetType,
    #[serde(default)]
    pub dependencies: HashSet<AssetID>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub license: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ShaderSource {
    pub kind: IRShaderSourceKind,
    pub origin: ShaderOrigin,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ShaderOrigin {
    Inline { code: String },
    External(SourceRef),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct UserShaderAsset {
    #[serde(default)]
    pub compile_options: Vec<String>,
    pub sources: Vec<ShaderSource>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct UserTextureAsset {
    pub sources: Vec<SourceRef>,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct UserAudioAsset {
    pub sample_rate: u32,
    pub channels: u8,
    pub source: SourceRef,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct UserMeshAsset {
    pub source: SourceRef,
    pub gen_material: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct UserMaterialAsset {
    pub base_color_factor: [f32; 4],
    pub base_color_texture: Option<SourceRef>,
    pub metallic_texture: Option<SourceRef>,
    #[serde(default)]
    pub metallic_factor: f32,
    pub roughness_texture: Option<SourceRef>,
    #[serde(default)]
    pub roughness_factor: f32,
    // pub normal: Option<NormalMap>,
    // pub occlusion: Option<Occlusion>,
    // pub emissive: Emissive,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum UserAssetProperties {
    Shader(UserShaderAsset),
    Texture(UserTextureAsset),
    Audio(UserAudioAsset),
    Material(UserMaterialAsset),
    Mesh(UserMeshAsset),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct UserAsset {
    pub header: UserAssetHeader,
    pub properties: UserAssetProperties,
}

impl DeepHash for ShaderSource {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> Result<(), String> {
        with_std(&self.kind, state);
        match &self.origin {
            ShaderOrigin::Inline { code } => {
                0u8.deep_hash(state, ctx)?;
                code.deep_hash(state, ctx)?;
            }
            ShaderOrigin::External(source) => {
                1u8.deep_hash(state, ctx)?;
                source.deep_hash(state, ctx)?;
            }
        }
        Ok(())
    }
}

impl DeepHash for UserShaderAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> Result<(), String> {
        self.compile_options.deep_hash(state, ctx)?;
        self.sources.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for UserTextureAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> Result<(), String> {
        self.sources.deep_hash(state, ctx)?;
        with_std(&self.pixel_format, state);
        self.use_mipmaps.deep_hash(state, ctx)?;
        with_std(&self.min_filter, state);
        with_std(&self.mag_filter, state);
        with_std(&self.texture_type, state);
        with_std(&self.wrap_s, state);
        with_std(&self.wrap_t, state);
        with_std(&self.wrap_r, state);
        Ok(())
    }
}

impl DeepHash for UserAudioAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> Result<(), String> {
        self.sample_rate.deep_hash(state, ctx)?;
        self.channels.deep_hash(state, ctx)?;
        self.source.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for UserMaterialAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> Result<(), String> {
        self.base_color_factor.deep_hash(state, ctx)?;
        self.base_color_texture.deep_hash(state, ctx)?;
        self.metallic_texture.deep_hash(state, ctx)?;
        self.metallic_factor.deep_hash(state, ctx)?;
        self.roughness_texture.deep_hash(state, ctx)?;
        self.roughness_factor.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for UserMeshAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> Result<(), String> {
        self.source.deep_hash(state, ctx)?;
        self.gen_material.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for UserAssetProperties {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> Result<(), String> {
        match self {
            UserAssetProperties::Shader(s) => {
                0u8.deep_hash(state, ctx)?;
                s.deep_hash(state, ctx)?;
            }
            UserAssetProperties::Texture(t) => {
                1u8.deep_hash(state, ctx)?;
                t.deep_hash(state, ctx)?;
            }
            UserAssetProperties::Audio(a) => {
                2u8.deep_hash(state, ctx)?;
                a.deep_hash(state, ctx)?;
            }
            UserAssetProperties::Material(m) => {
                3u8.deep_hash(state, ctx)?;
                m.deep_hash(state, ctx)?;
            }
            UserAssetProperties::Mesh(m) => {
                4u8.deep_hash(state, ctx)?;
                m.deep_hash(state, ctx)?;
            }
        }
        Ok(())
    }
}

impl DeepHash for AssetID {
    fn deep_hash<T: Hasher>(&self, state: &mut T, _: &mut DeepHashCtx) -> Result<(), String> {
        self.as_str().hash(state);
        Ok(())
    }
}

impl DeepHash for UserAssetHeader {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> Result<(), String> {
        with_std(&self.asset_type, state);
        self.dependencies.deep_hash(state, ctx)?;
        self.tags.deep_hash(state, ctx)?;
        self.author.deep_hash(state, ctx)?;
        self.license.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for UserAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> Result<(), String> {
        self.header.deep_hash(state, ctx)?;
        self.properties.deep_hash(state, ctx)?;
        Ok(())
    }
}
