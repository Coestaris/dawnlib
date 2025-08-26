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
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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

    let mesh_id = normalize_name(file.path.clone());
    if scenes.len() > 1 {
        return Err(format!(
            "Mesh file '{}' contains multiple scenes",
            path.display()
        ));
    }
    let scene = &scenes[0];

    let mut all_extras = Vec::new();
    if let Some(extras) = scene.extras.as_ref() {
        all_extras.push(extras);
    }

    let common_deps = Arc::new(Mutex::new(HashSet::new()));

    struct ModelProcessResult {
        irs: Vec<PartialIR>,
        mesh: IRSubMesh,
    }

    let results = scene
        .models
        .par_iter()
        .enumerate()
        .map(|(i, model)| -> Result<ModelProcessResult, String> {
            let mut min = Vec3::splat(f32::MAX);
            let mut max = Vec3::splat(f32::MIN);
            let mut irs = Vec::new();

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
                if common_deps.lock().unwrap().insert(material_id.clone()) {
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
                    irs.extend(
                        convert_material_from_memory(material_id.clone(), material_header, user)
                            .map_err(|e| format!("Failed to convert material: {}", e))?,
                    );
                }
                material_id
            } else {
                AssetID::default()
            };

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

            Ok(ModelProcessResult {
                irs,
                mesh: IRSubMesh {
                    vertices: data,
                    indices: model.indices().unwrap().clone(),
                    material: material_id,
                    bounds: IRMeshBounds {
                        min: min.to_array(),
                        max: max.to_array(),
                    },
                    primitive: primitive_type,
                    primitives_count,
                },
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<Vec<_>>();

    let header = file.asset.header.clone();
    let mut irs = Vec::new();
    let mut submesh = Vec::with_capacity(results.len());
    let mut min_global = Vec3::splat(f32::MAX);
    let mut max_global = Vec3::splat(f32::MIN);
    for result in results {
        irs.extend(result.irs);
        min_global = min_global.min(result.mesh.bounds.min.into());
        max_global = max_global.max(result.mesh.bounds.max.into());
        submesh.push(result.mesh);
    }

    irs.push(PartialIR::new_from_id(
        IRAsset::Mesh(IRMesh {
            submesh,
            bounds: IRMeshBounds {
                min: min_global.to_array(),
                max: max_global.to_array(),
            },
        }),
        header,
        mesh_id.clone(),
    ));

    Ok(irs)
}
