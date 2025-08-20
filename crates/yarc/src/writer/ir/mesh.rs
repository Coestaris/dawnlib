use crate::writer::ir::material::{convert_material_from_memory, UserMaterialAssetInner};
use crate::writer::ir::{normalize_name, PartialIR};
use crate::writer::user::{UserAssetHeader, UserMaterialAsset, UserMeshAsset};
use crate::writer::UserAssetFile;
use dawn_assets::ir::mesh::{IRMesh, IRMeshBounds, IRPrimitive, IRVertex};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetID, AssetType};
use easy_gltf::model::Mode;
use glam::Vec3;
use log::{debug, info};
use std::path::PathBuf;

pub fn convert_mesh(file: &UserAssetFile, user: &UserMeshAsset) -> Result<Vec<PartialIR>, String> {
    debug!("Converting mesh: {:?}", file);

    // Try to find the file in the same directory as the shader
    let parent = file.path.parent().unwrap();
    let mesh = PathBuf::from(user.file.clone());
    let mesh = parent.join(mesh);

    let scenes = easy_gltf::load(&mesh)
        .map_err(|e| format!("Failed to load mesh file '{}': {}", mesh.display(), e))?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut primitives_count = 0;
    let mut primitive_type = None;
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut materials = Vec::new();

    let mut all_extras = Vec::new();
    for scene in &scenes {
        if let Some(extras) = scene.extras.as_ref() {
            all_extras.push(extras);
        }

        for model in &scene.models {
            materials.push(model.material().clone());

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

    let mut result = Vec::new();
    let mesh_id = normalize_name(mesh.clone());
    let mut header = file.asset.header.clone();
    if let Some(material_id) = &user.gen_material {
        if materials.len() > 1 {
            return Err(format!(
                "Mesh '{}' has more than one material, but only one material can be generated",
                mesh.display()
            ));
        }
        if let Some(material) = materials.first() {
            header.dependencies.push(material_id.clone());

            let header = UserAssetHeader {
                asset_type: AssetType::Material,
                dependencies: vec![],
                tags: vec![],
                author: Some("Auto-generated".to_string()),
                license: None,
            };
            let user = UserMaterialAssetInner {
                base_color_factor: *material.pbr.base_color_factor.as_ref(),
                base_color_texture: material.pbr.base_color_texture.clone(),
                metallic_texture: material.pbr.metallic_texture.clone(),
                metallic_factor: material.pbr.metallic_factor,
                roughness_texture: material.pbr.roughness_texture.clone(),
                roughness_factor: material.pbr.roughness_factor,
            };
            result.extend(
                convert_material_from_memory(material_id.clone(), header, user)
                    .map_err(|e| format!("Failed to convert material: {}", e))?,
            );
        }
    }

    // TODO: Handle extras
    info!("Extras: {:?}", all_extras);

    result.push(PartialIR::new_from_id(
        IRAsset::Mesh(IRMesh {
            vertices,
            indices,
            material: AssetID::default(),
            bounds: IRMeshBounds {
                min: min.to_array(),
                max: max.to_array(),
            },
            primitive: primitive_type.unwrap_or(IRPrimitive::Points),
            primitives_count,
        }),
        header,
        mesh_id.clone(),
    ));

    Ok(result)
}
