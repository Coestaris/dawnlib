use crate::writer::ir::audio::convert_audio;
use crate::writer::ir::material::convert_material;
use crate::writer::ir::mesh::convert_mesh;
use crate::writer::ir::shader::convert_shader;
use crate::writer::ir::texture::convert_texture;
use crate::writer::user::{UserAssetHeader, UserAssetProperties};
use crate::writer::{UserAssetFile, UserIRAsset, WriterError};
use crate::ChecksumAlgorithm;
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetChecksum, AssetHeader, AssetID};
use std::path::PathBuf;

mod audio;
mod material;
mod mesh;
mod shader;
mod texture;

fn checksum<T>(obj: &T, algorithm: ChecksumAlgorithm) -> Result<AssetChecksum, WriterError> {
    // Transmute object to a byte slice
    let slice =
        unsafe { std::slice::from_raw_parts(obj as *const T as *const u8, size_of_val(obj)) };

    let hash = match algorithm {
        ChecksumAlgorithm::Md5 => {
            let mut hasher = md5::Context::new();
            hasher.consume(slice);
            hasher.finalize().0
        }
        _ => {
            return Err(WriterError::UnsupportedChecksumAlgorithm(algorithm));
        }
    };

    Ok(AssetChecksum::from_bytes(&hash))
}

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

fn partial_to_ir(
    partial_ir: PartialIR,
    algorithm: ChecksumAlgorithm,
) -> Result<UserIRAsset, String> {
    Ok(UserIRAsset {
        header: AssetHeader {
            id: partial_ir.id,
            tags: partial_ir.header.tags.clone(),
            author: partial_ir.header.author.clone(),
            asset_type: partial_ir.header.asset_type,
            checksum: checksum(&partial_ir.ir, algorithm)
                .map_err(|e| format!("Failed to compute checksum: {}", e))?,
            dependencies: partial_ir.header.dependencies.clone(),
            license: partial_ir.header.license.clone(),
        },
        ir: partial_ir.ir,
    })
}

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
}

pub fn user_to_ir(
    file: UserAssetFile,
    algorithm: ChecksumAlgorithm,
) -> Result<Vec<UserIRAsset>, String> {
    let irs = match &file.asset.properties {
        UserAssetProperties::Shader(shader) => convert_shader(&file, shader)?,
        UserAssetProperties::Texture(texture) => convert_texture(&file, texture)?,
        UserAssetProperties::Audio(audio) => convert_audio(&file, audio)?,
        UserAssetProperties::Mesh(mesh) => convert_mesh(&file, mesh)?,
        UserAssetProperties::Material(material) => convert_material(&file, material)?,
    };

    let mut result = Vec::new();
    for ir in irs {
        result.push(partial_to_ir(ir, algorithm)?);
    }

    Ok(result)
}
