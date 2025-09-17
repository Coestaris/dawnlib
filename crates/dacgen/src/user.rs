use crate::deep_hash::{with_std, DeepHash, DeepHashCtx};
use crate::source::SourceRef;
use dawn_assets::ir::dictionary::IRDictionaryEntry;
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
pub(crate) struct UserMaterialAsset {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct CharSet {
    /// Include characters from '0' to '9'
    pub numbers: bool,
    /// Include characters from '!' to '/' and ':' to '@' and '[' to '`' and '{' to '~'
    pub special_symbols: bool,
    /// Include characters [a-zA-Z]
    pub latin: bool,
    /// Include characters [а-яА-Я]
    pub cyrillic: bool,
}

impl CharSet {
    pub fn to_chars(&self) -> HashSet<char> {
        let mut chars = HashSet::new();
        if self.numbers {
            for c in '0'..='9' {
                chars.insert(c);
            }
        }
        if self.latin {
            for c in 'a'..='z' {
                chars.insert(c);
            }
            for c in 'A'..='Z' {
                chars.insert(c);
            }
        }

        if self.cyrillic {
            for c in 'а'..='Я' {
                chars.insert(c);
            }
            for c in 'А'..='Я' {
                chars.insert(c);
            }

            // Include Ukrainian characters
            chars.insert('і');
            chars.insert('І');
            chars.insert('ї');
            chars.insert('Ї');
            chars.insert('є');
            chars.insert('Є');
        }

        if self.special_symbols {
            chars.insert('!');
            chars.insert('\"');
            chars.insert('#');
            chars.insert('$');
            chars.insert('%');
            chars.insert('&');
            chars.insert('\'');
            chars.insert('(');
            chars.insert(')');
            chars.insert('*');
            chars.insert('+');
            chars.insert(',');
            chars.insert('-');
            chars.insert('.');
            chars.insert('/');
            chars.insert(':');
            chars.insert(';');
            chars.insert('<');
            chars.insert('=');
            chars.insert('>');
            chars.insert('?');
            chars.insert('@');
            chars.insert('[');
            chars.insert('\\');
            chars.insert(']');
            chars.insert('^');
            chars.insert('_');
            chars.insert('`');
            chars.insert('{');
            chars.insert('|');
            chars.insert('}');
            chars.insert('~');
        }

        chars
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct UserFontAsset {
    pub source: SourceRef,
    pub charset: CharSet,
    pub size: u32,

    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct UserDictionaryAsset {
    #[serde(default)]
    pub entries: Vec<IRDictionaryEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct UserBlobAsset {
    pub source: SourceRef,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum UserAssetProperties {
    Shader(UserShaderAsset),
    Texture(UserTextureAsset),
    Audio(UserAudioAsset),
    Material(UserMaterialAsset),
    Mesh(UserMeshAsset),
    Font(UserFontAsset),
    Dictionary(UserDictionaryAsset),
    Blob(UserBlobAsset),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct UserAsset {
    pub header: UserAssetHeader,
    pub properties: UserAssetProperties,
}

impl DeepHash for UserBlobAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.source.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for ShaderOrigin {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        match &self {
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

impl DeepHash for ShaderSource {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        with_std(&self.kind, state);
        self.origin.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for UserShaderAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.compile_options.deep_hash(state, ctx)?;
        self.sources.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for UserTextureAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
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
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.sample_rate.deep_hash(state, ctx)?;
        self.channels.deep_hash(state, ctx)?;
        self.source.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for UserMaterialAsset {
    fn deep_hash<T: Hasher>(&self, _state: &mut T, _ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        Ok(())
    }
}

impl DeepHash for UserMeshAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.source.deep_hash(state, ctx)?;
        self.gen_material.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for CharSet {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.numbers.deep_hash(state, ctx)?;
        self.special_symbols.deep_hash(state, ctx)?;
        self.latin.deep_hash(state, ctx)?;
        self.cyrillic.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for UserFontAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.source.deep_hash(state, ctx)?;
        self.charset.deep_hash(state, ctx)?;
        self.size.deep_hash(state, ctx)?;
        self.bold.deep_hash(state, ctx)?;
        self.italic.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for IRDictionaryEntry {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        match self {
            IRDictionaryEntry::String(v) => {
                1u8.deep_hash(state, ctx)?;
                v.deep_hash(state, ctx)?;
            }
            IRDictionaryEntry::Int(v) => {
                2u8.deep_hash(state, ctx)?;
                v.deep_hash(state, ctx)?;
            }
            IRDictionaryEntry::UInt(v) => {
                3u8.deep_hash(state, ctx)?;
                v.deep_hash(state, ctx)?;
            }
            IRDictionaryEntry::F32(v) => {
                4u8.deep_hash(state, ctx)?;
                v.deep_hash(state, ctx)?;
            }
            IRDictionaryEntry::Bool(v) => {
                5u8.deep_hash(state, ctx)?;
                v.deep_hash(state, ctx)?;
            }
            IRDictionaryEntry::Map(v) => {
                6u8.deep_hash(state, ctx)?;
                v.deep_hash(state, ctx)?;
            }
            IRDictionaryEntry::Array(v) => {
                7u8.deep_hash(state, ctx)?;
                v.deep_hash(state, ctx)?;
            }
            IRDictionaryEntry::Vec2f(v) => {
                8u8.deep_hash(state, ctx)?;
                v.deep_hash(state, ctx)?;
            }
            IRDictionaryEntry::Vec3f(v) => {
                9u8.deep_hash(state, ctx)?;
                v.deep_hash(state, ctx)?;
            }
            IRDictionaryEntry::Vec4f(v) => {
                10u8.deep_hash(state, ctx)?;
                v.deep_hash(state, ctx)?;
            }
            IRDictionaryEntry::Mat3f(v) => {
                11u8.deep_hash(state, ctx)?;
                v.deep_hash(state, ctx)?;
            }
            IRDictionaryEntry::Mat4f(v) => {
                12u8.deep_hash(state, ctx)?;
                v.deep_hash(state, ctx)?;
            }
        }

        Ok(())
    }
}

impl DeepHash for UserDictionaryAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.entries.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for UserAssetProperties {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
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
            UserAssetProperties::Font(f) => {
                5u8.deep_hash(state, ctx)?;
                f.deep_hash(state, ctx)?;
            }
            UserAssetProperties::Dictionary(d) => {
                6u8.deep_hash(state, ctx)?;
                d.deep_hash(state, ctx)?;
            }
            UserAssetProperties::Blob(b) => {
                7u8.deep_hash(state, ctx)?;
                b.deep_hash(state, ctx)?;
            }
        }
        Ok(())
    }
}

impl DeepHash for AssetID {
    fn deep_hash<T: Hasher>(&self, state: &mut T, _: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.as_str().hash(state);
        Ok(())
    }
}

impl DeepHash for UserAssetHeader {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        with_std(&self.asset_type, state);
        self.dependencies.deep_hash(state, ctx)?;
        self.tags.deep_hash(state, ctx)?;
        self.author.deep_hash(state, ctx)?;
        self.license.deep_hash(state, ctx)?;
        Ok(())
    }
}

impl DeepHash for UserAsset {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.header.deep_hash(state, ctx)?;
        self.properties.deep_hash(state, ctx)?;
        Ok(())
    }
}
