use crate::writer::user::{
    UserAsset, UserAssetProperties, UserAudioAsset, UserShaderAsset, UserTextureAsset,
};
use crate::{ChecksumAlgorithm, PackedAsset, WriterError};
use std::collections::HashMap;
use std::path::Path;
use yage2_core::assets::raw::{AssetRaw, AudioAssetRaw, ShaderAssetRaw, TextureAssetRaw};
use yage2_core::assets::{AssetChecksum, AssetHeader};

fn checksum<T>(obj: &T, algorithm: ChecksumAlgorithm) -> Result<AssetChecksum, WriterError> {
    // Transmute object to a byte slice
    let slice = unsafe {
        std::slice::from_raw_parts(obj as *const T as *const u8, std::mem::size_of_val(obj))
    };

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

fn with_checksum(
    obj: &AssetRaw,
    algorithm: ChecksumAlgorithm,
    header: &AssetHeader,
) -> Result<AssetHeader, String> {
    let checksum =
        checksum(obj, algorithm).map_err(|e| format!("Failed to compute checksum: {}", e))?;
    Ok(AssetHeader {
        id: header.id.clone(),
        tags: header.tags.clone(),
        asset_type: header.asset_type,
        checksum,
    })
}

pub fn user_asset_to_raw(
    asset_path: &Path,
    user: &UserAsset,
    algorithm: ChecksumAlgorithm,
) -> Result<(AssetHeader, AssetRaw), String> {
    let raw = match &user.properties {
        UserAssetProperties::Shader(shader) => {
            AssetRaw::Shader(user_shader_to_raw(asset_path, shader)?)
        }
        UserAssetProperties::Texture(texture) => {
            AssetRaw::Texture(user_texture_to_raw(asset_path, texture)?)
        }
        UserAssetProperties::Audio(audio) => AssetRaw::Audio(user_audio_to_raw(asset_path, audio)?),
    };

    Ok((with_checksum(&raw, algorithm, &user.header)?, raw))
}

pub fn user_shader_to_raw(
    asset_path: &Path,
    user: &UserShaderAsset,
) -> Result<ShaderAssetRaw, String> {
    let mut sources = HashMap::new();
    for (source_type, path_part) in user.files.iter() {
        // Try to find the file in the same directory as the shader
        let directory = asset_path.parent().unwrap();
        let path = directory.join(path_part);

        let content = std::fs::read(path.clone()).map_err(|e| {
            format!(
                "Failed to read shader source file '{}': {}",
                path.to_string_lossy(),
                e
            )
        })?;
        sources.insert(source_type.clone(), content);
    }

    Ok(ShaderAssetRaw {
        sources,
        compile_options: user.compile_options.clone(),
    })
}

pub fn user_texture_to_raw(
    asset_path: &Path,
    user: &UserTextureAsset,
) -> Result<TextureAssetRaw, String> {
    todo!()
}

pub fn user_audio_to_raw(
    asset_path: &Path,
    user: &UserAudioAsset,
) -> Result<AudioAssetRaw, String> {
    todo!()
}
