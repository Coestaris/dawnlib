use crate::writer::ir::audio::convert_audio;
use crate::writer::ir::mesh::convert_mesh;
use crate::writer::ir::shader::convert_shader;
use crate::writer::ir::texture::convert_texture;
use crate::writer::user::{UserAsset, UserAssetHeader, UserAssetProperties};
use crate::writer::{UserAssetFile, UserIRAsset};
use crate::{ChecksumAlgorithm, WriterError};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetChecksum, AssetHeader};
use std::path::{Path, PathBuf};

mod audio;
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
fn normalize_name<P: AsRef<std::path::Path>>(path: P) -> String {
    // Get rid of the extension and normalize the name
    let name = path
        .as_ref()
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_lowercase();

    // Replace whitespace with underscores and remove special characters
    name.replace('.', "_")
        .replace(' ', "_")
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "")
}

fn user_header_to_asset(
    id: 
    obj: &IRAsset,
    algorithm: ChecksumAlgorithm,
    header: &UserAssetHeader,
) -> Result<AssetHeader, String> {
    let checksum =
        checksum(obj, algorithm).map_err(|e| format!("Failed to compute checksum: {}", e))?;
    Ok(AssetHeader {
        id: path_to_asset_id(asset_path).into(),
        tags: header.tags.clone(),
        author: header.author.clone(),
        asset_type: header.asset_type,
        checksum,
        dependencies: header.dependencies.clone(),
        license: header.license.clone(),
    })
}

pub(crate) struct PartialIR {
    name: String,
    header: UserAssetHeader,
    ir: IRAsset,
}

pub fn user_to_ir(
    file: UserAssetFile,
    algorithm: ChecksumAlgorithm,
) -> Result<Vec<UserIRAsset>, String> {
    let irs = match &file.asset.properties {
        UserAssetProperties::Shader(shader) => IRAsset::Shader(convert_shader(&file, shader)?),
        UserAssetProperties::Texture(texture) => IRAsset::Texture(convert_texture(&file, texture)?),
        UserAssetProperties::Audio(audio) => IRAsset::Audio(convert_audio(&file, audio)?),
        UserAssetProperties::Mesh(mesh) => IRAsset::Mesh(convert_mesh(&file, mesh)?),
    };

    let mut result = Vec::new();
    for ir in irs {
        result.push(
            UserIRAsset {
                header: user_header_to_asset(&file.path, &ir, algorithm, &file.asset.header)
                    .map_err(|e| format!("Failed to convert user header to asset: {}", e))?,
                ir,
            },
        );
    }

    Ok(result)
}
