use crate::writer::pix::repack;
use crate::writer::user::{
    UserAsset, UserAssetProperties, UserAudioAsset, UserShaderAsset, UserTextureAsset,
};
use crate::{ChecksumAlgorithm, WriterError};
use dawn_assets::ir::audio::IRAudio;
use dawn_assets::ir::shader::IRShader;
use dawn_assets::ir::texture::{IRTexture, IRTextureType};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetChecksum, AssetHeader};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

fn with_checksum(
    obj: &IRAsset,
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
        dependencies: header.dependencies.clone(),
    })
}

pub fn user_to_ir(
    asset_path: &Path,
    user: &UserAsset,
    algorithm: ChecksumAlgorithm,
) -> Result<(AssetHeader, IRAsset), String> {
    let ir = match &user.properties {
        UserAssetProperties::Shader(shader) => {
            IRAsset::Shader(user_shader_to_ir(asset_path, shader)?)
        }
        UserAssetProperties::Texture(texture) => {
            IRAsset::Texture(user_texture_to_ir(asset_path, texture)?)
        }
        UserAssetProperties::Audio(audio) => IRAsset::Audio(user_audio_to_ir(asset_path, audio)?),
    };

    Ok((with_checksum(&ir, algorithm, &user.header)?, ir))
}

pub fn user_shader_to_ir(asset_path: &Path, user: &UserShaderAsset) -> Result<IRShader, String> {
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

    Ok(IRShader {
        sources,
        compile_options: user.compile_options.clone(),
    })
}

pub fn user_texture_to_ir(asset_path: &Path, user: &UserTextureAsset) -> Result<IRTexture, String> {
    // Try to find the file in the same directory as the shader
    let parent = asset_path.parent().unwrap();
    let file = PathBuf::from(user.files[0].clone());
    let file = parent.join(file);

    let img = match image::open(&file) {
        Ok(img) => img,
        Err(e) => {
            return Err(format!(
                "Failed to load texture image '{}': {}",
                file.display(),
                e
            ))
        }
    };

    let texture_type = match user.texture_type {
        IRTextureType::Unknown => IRTextureType::Texture2D {
            width: img.width(),
            height: img.height(),
        },
        any => any,
    };

    Ok(IRTexture {
        data: repack(img, user.pixel_format, texture_type)?,
        texture_type: texture_type.clone(),
        pixel_format: user.pixel_format.clone(),
        use_mipmaps: user.use_mipmaps,
        min_filter: user.min_filter.clone(),
        mag_filter: user.mag_filter.clone(),
        wrap_s: user.wrap_s.clone(),
        wrap_t: user.wrap_t.clone(),
        wrap_r: user.wrap_r.clone(),
    })
}

pub fn user_audio_to_ir(asset_path: &Path, user: &UserAudioAsset) -> Result<IRAudio, String> {
    todo!()
}
