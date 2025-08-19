use crate::writer::ir::PartialIR;
use crate::writer::user::{UserAssetHeader, UserMeshAsset};
use crate::writer::UserAssetFile;
use dawn_assets::ir::mesh::{IRMesh, IRMeshBounds, IRPrimitive, IRVertex};
use dawn_assets::AssetID;
use easy_gltf::model::Mode;
use glam::Vec3;
use log::info;
use std::path::{Path, PathBuf};

pub fn convert_mesh(file: &UserAssetFile, user: &UserMeshAsset) -> Result<Vec<PartialIR>, String> {
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

    let mut all_extras = Vec::new();
    for scene in &scenes {
        if let Some(extras) = scene.extras.as_ref() {
            all_extras.push(extras);
        }

        for model in &scene.models {
            if let Some(primitive_extras) = model.primitive_extras().as_ref() {
                all_extras.push(primitive_extras);
            }
            if let Some(mesh_extras) = model.mesh_extras().as_ref() {
                all_extras.push(mesh_extras);
            }

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

    // TODO: Handle extras
    info!("Extras: {:?}", all_extras);

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
