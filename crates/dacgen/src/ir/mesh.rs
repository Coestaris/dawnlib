use crate::ir::material::{convert_material_from_memory, UserMaterialAssetInner};
use crate::ir::{normalize_name, PartialIR};
use crate::user::{UserAssetHeader, UserMeshAsset};
use crate::UserAssetFile;
use dawn_assets::ir::mesh::{IRMesh, IRMeshBounds, IRPrimitive, IRSubMesh, IRVertex};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetHeader, AssetID, AssetType};
use easy_gltf::model::Mode;
use glam::Vec3;
use log::{debug, info};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn convert_mesh(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserMeshAsset,
) -> Result<Vec<PartialIR>, String> {
    // Try to find the file in the same directory as the shader
    let path = user.source.as_path(cache_dir, cwd)?;
    let scenes = easy_gltf::load(&path)
        .map_err(|e| format!("Failed to load mesh file '{}': {}", path.display(), e))?;

    let mut global_min = Vec3::splat(f32::MAX);
    let mut global_max = Vec3::splat(f32::MIN);

    let mesh_id = normalize_name(file.path.clone());
    let mut header = file.asset.header.clone();
    let mut result = Vec::new();
    let mut submesh = Vec::new();

    let mut all_extras = Vec::new();
    for scene in &scenes {
        if let Some(extras) = scene.extras.as_ref() {
            all_extras.push(extras);
        }

        for (i, model) in scene.models.iter().enumerate() {
            let mut min = global_min;
            let mut max = global_max;

            let material_id = if user.gen_material {
                let material = model.material();
                let material_id = match (material.name.clone(), model.mesh_name()) {
                    (Some(name), _) => format!("_{}_{}_material", mesh_id.as_str(), name),
                    (None, Some(name)) => {
                        format!("_{}_{}_material", mesh_id.as_str(), name)
                    }
                    (None, None) => format!("_{}_{}_material", mesh_id.as_str(), i),
                };

                let material_id = AssetID::new(material_id);

                // Mesh reuses material. Do not generate it again.
                if header.dependencies.insert(material_id.clone()) {
                    let material_header = UserAssetHeader {
                        asset_type: AssetType::Material,
                        dependencies: HashSet::new(),
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
                        convert_material_from_memory(material_id.clone(), material_header, user)
                            .map_err(|e| format!("Failed to convert material: {}", e))?,
                    );
                }
                material_id
            } else {
                AssetID::default()
            };

            if let Some(primitive_extras) = model.primitive_extras().as_ref() {
                all_extras.push(primitive_extras);
            }
            if let Some(mesh_extras) = model.mesh_extras().as_ref() {
                all_extras.push(mesh_extras);
            }

            let (primitive_type, primitives_count) = match model.mode() {
                Mode::Points => (IRPrimitive::Points, model.indices().unwrap().len()),
                Mode::Lines => (IRPrimitive::Lines, model.indices().unwrap().len() / 2),
                Mode::Triangles => (IRPrimitive::Triangles, model.indices().unwrap().len() / 3),
                _ => {
                    unimplemented!()
                }
            };

            let mut data = Vec::with_capacity(model.vertices().len() * size_of::<IRVertex>());
            for vertex in model.vertices() {
                let position = vertex.position.as_ref();
                let vec = Vec3::from(*position);
                min = min.min(vec);
                max = max.max(vec);

                data.extend(
                    IRVertex {
                        position: *position,
                        normal: *vertex.normal.as_ref(),
                        tex_coord: *vertex.tex_coords.as_ref(),
                    }
                    .into_bytes(),
                );
            }

            submesh.push(IRSubMesh {
                vertices: data,
                indices: model.indices().unwrap().clone(),
                material: material_id,
                bounds: IRMeshBounds {
                    min: min.to_array(),
                    max: max.to_array(),
                },
                primitive: primitive_type,
                primitives_count,
            });

            global_min = global_min.min(min);
            global_max = global_max.max(max);
        }
    }

    // TODO: Handle extras
    info!("Extras: {:?}", all_extras);

    result.push(PartialIR::new_from_id(
        IRAsset::Mesh(IRMesh {
            submesh,
            bounds: IRMeshBounds {
                min: global_min.to_array(),
                max: global_max.to_array(),
            },
        }),
        header,
        mesh_id.clone(),
    ));

    Ok(result)
}
