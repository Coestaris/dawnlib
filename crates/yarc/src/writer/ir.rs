use crate::writer::pix::repack;
use crate::writer::user::{
    UserAsset, UserAssetProperties, UserAudioAsset, UserMeshAsset, UserShaderAsset,
    UserTextureAsset,
};
use crate::{ChecksumAlgorithm, WriterError};
use dawn_assets::ir::audio::IRAudio;
use dawn_assets::ir::mesh::{IRMesh, IRMeshBounds, IRPrimitive, IRVertex};
use dawn_assets::ir::shader::IRShader;
use dawn_assets::ir::texture::{IRTexture, IRTextureType};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetChecksum, AssetHeader, AssetID};
use easy_gltf;
use easy_gltf::model::Mode;
use glam::Vec3;
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
        UserAssetProperties::Mesh(mesh) => IRAsset::Mesh(user_mesh_to_ir(asset_path, mesh)?),
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

pub fn user_mesh_to_ir(asset_path: &Path, user: &UserMeshAsset) -> Result<IRMesh, String> {
    // Try to find the file in the same directory as the shader
    let parent = asset_path.parent().unwrap();
    let file = PathBuf::from(user.file.clone());
    let file = parent.join(file);

    let scenes = easy_gltf::load(&file)
        .map_err(|e| format!("Failed to load mesh file '{}': {}", file.display(), e))?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut primitives_count = 0;
    let mut primitive_type = None;
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);

    for scene in &scenes {
        for model in &scene.models {
            let new_type = match model.mode() {
                Mode::Points => {
                    primitives_count += model.indices().unwrap().len();
                    IRPrimitive::Points
                }
                Mode::Lines => {
                    primitives_count += model.indices().unwrap().len() / 2;
                    IRPrimitive::Lines
                }
                Mode::Triangles => {
                    primitives_count += model.indices().unwrap().len() / 3;
                    IRPrimitive::Triangles
                }
                _ => {
                    unimplemented!()
                }
            };
            if let Some(primitive_type) = primitive_type.take() {
                if primitive_type != new_type {
                    return Err(format!(
                        "Inconsistent primitive types in mesh: {:?} vs {:?}",
                        primitive_type, new_type
                    ));
                }
            } else {
                primitive_type = Some(new_type);
            }

            for vertex in model.vertices() {
                let position = vertex.position.as_ref();
                let vec = Vec3::from(*position);
                min = min.min(vec);
                max = max.max(vec);

                vertices.push(IRVertex {
                    position: *position,
                    normal: *vertex.normal.as_ref(),
                    tex_coord: *vertex.tex_coords.as_ref(),
                });
            }
            for index in model.indices().unwrap() {
                indices.push(*index);
            }
        }
    }

    Ok(IRMesh {
        vertices,
        indices,
        material: AssetID::default(),
        bounds: IRMeshBounds {
            min: min.to_array(),
            max: max.to_array(),
        },
        primitive: primitive_type.unwrap_or(IRPrimitive::Points),
        primitives_count,
    })
}
