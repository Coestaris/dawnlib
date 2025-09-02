use crate::ir::audio::convert_audio;
use crate::ir::dictionary::convert_dictionary;
use crate::ir::font::convert_font;
use crate::ir::material::convert_material;
use crate::ir::mesh::convert_mesh;
use crate::ir::shader::convert_shader;
use crate::ir::texture::convert_texture;
use crate::user::{UserAssetHeader, UserAssetProperties};
use crate::{ChecksumAlgorithm, UserAssetFile, UserIRAsset};
use anyhow::Context;
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetChecksum, AssetHeader, AssetID};
use dawn_util::profile::Measure;
use std::path::{Path, PathBuf};

mod audio;
mod dictionary;
mod font;
mod material;
mod mesh;
mod shader;
mod texture;

/// Normalize the file name by removing the extension, converting to lowercase,
/// replacing whitespace with underscores, and removing special characters.
pub fn normalize_name(path: PathBuf) -> AssetID {
    // Get rid of the extension and normalize the name
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_lowercase();

    // Replace whitespace with underscores and remove special characters
    name.replace('.', "_")
        .replace(' ', "_")
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "")
        .into()
}

#[derive(Debug)]
pub(crate) struct PartialIR {
    id: AssetID,
    header: UserAssetHeader,
    ir: IRAsset,
}

impl PartialIR {
    pub fn new_from_path(ir: IRAsset, header: UserAssetHeader, path: PathBuf) -> Self {
        Self {
            id: normalize_name(path.to_path_buf()),
            header,
            ir,
        }
    }

    pub fn new_from_id(ir: IRAsset, header: UserAssetHeader, id: AssetID) -> Self {
        Self { id, header, ir }
    }

    pub fn convert(self, algorithm: ChecksumAlgorithm) -> anyhow::Result<UserIRAsset> {
        Ok(UserIRAsset {
            header: AssetHeader {
                id: self.id,
                tags: self.header.tags.clone(),
                author: self.header.author.clone(),
                asset_type: self.header.asset_type,
                checksum: AssetChecksum::default(), // TODO: Implement checksum calculation
                dependencies: self.header.dependencies.clone(),
                license: self.header.license.clone(),
            },
            ir: self.ir,
        })
    }
}

impl UserAssetFile {
    pub fn convert(
        &self,
        cache_dir: &Path,
        cwd: &Path,
        algorithm: ChecksumAlgorithm,
    ) -> anyhow::Result<Vec<UserIRAsset>> {
        let _measure = Measure::new(format!(
            "Converted user asset {} to IR",
            self.path.display()
        ));

        let irs = match &self.asset.properties {
            UserAssetProperties::Shader(shader) => convert_shader(self, cache_dir, cwd, shader),
            UserAssetProperties::Texture(texture) => convert_texture(self, cache_dir, cwd, texture),
            UserAssetProperties::Audio(audio) => convert_audio(self, cache_dir, cwd, audio),
            UserAssetProperties::Mesh(mesh) => convert_mesh(self, cache_dir, cwd, mesh),
            UserAssetProperties::Material(mat) => convert_material(self, cache_dir, cwd, mat),
            UserAssetProperties::Font(font) => convert_font(self, cache_dir, cwd, font),
            UserAssetProperties::Dictionary(dict) => convert_dictionary(self, cache_dir, cwd, dict),
        }
        .with_context(|| format!("Failed to convert asset {}", self.path.display()))?;

        let mut result = Vec::new();
        for ir in irs {
            result.push(ir.convert(algorithm)?);
        }

        Ok(result)
    }
}
