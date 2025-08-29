use crate::ir::{normalize_name, PartialIR};
use crate::user::{UserAssetHeader, UserMeshAsset};
use crate::UserAssetFile;
use dawn_assets::ir::material::IRMaterial;
use dawn_assets::ir::mesh::{IRIndexType, IRMesh, IRMeshBounds, IRSubMesh, IRTopology, IRVertex};
use dawn_assets::ir::texture::{IRPixelFormat, IRTexture, IRTextureType};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetID, AssetType};
use dawn_util::profile::Measure;
use glam::{Mat4, Vec3};
use gltf::buffer::Data;
use gltf::image::Format;
use gltf::mesh::Mode;
use gltf::scene::Transform;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

pub fn transform_to_matrix(transform: Transform) -> Mat4 {
    let tr = transform.matrix();
    Mat4::from_cols_array(&[
        tr[0][0], tr[0][1], tr[0][2], tr[0][3], tr[1][0], tr[1][1], tr[1][2], tr[1][3], tr[2][0],
        tr[2][1], tr[2][2], tr[2][3], tr[3][0], tr[3][1], tr[3][2], tr[3][3],
    ])
}

#[derive(Clone)]
struct ProcessCtx<'a> {
    buffers: &'a Vec<Data>,
    index_type: IRIndexType,
    images: &'a Vec<gltf::image::Data>,
    processed_materials: Arc<Mutex<HashMap<usize, AssetID>>>,
    processed_textures: Arc<Mutex<HashMap<usize, AssetID>>>,
    mesh_id: AssetID,
}

struct MeshWrap<'a> {
    transform: Mat4,
    node: gltf::Mesh<'a>,
    ctx: ProcessCtx<'a>,
}

struct PrimitiveProcessResult {
    irs: Vec<PartialIR>,
    mesh: IRSubMesh,
}

#[derive(Debug, Clone, Error)]
enum MeshError {
    #[error("Mesh asset source is not a file: {0}")]
    NotAFile(String),
    #[error("Failed to load GLTF or GLB file: {0}")]
    LoadError(String),
    #[error("Mesh file does not contain any scene")]
    NoScene,
    #[error("Mesh file contains multiple scenes")]
    MultipleScenes(usize),
    #[error("Material {material_id} is missing texture source for {texture_type:?} texture index {texture_index}"
    )]
    MissingTextureSource {
        material_id: AssetID,
        texture_type: MaterialTextureType,
        texture_index: usize,
    },
    #[error("Primitive {mesh_index}:{primitive_index} is missing indices")]
    MissingIndices {
        mesh_index: usize,
        primitive_index: usize,
    },
    #[error("Primitive {mesh_index}:{primitive_index} is missing positions")]
    MissingPositions {
        mesh_index: usize,
        primitive_index: usize,
    },
    #[error("Primitive {mesh_index}:{primitive_index} is missing normals")]
    MissingNormals {
        mesh_index: usize,
        primitive_index: usize,
    },
    #[error("Primitive {mesh_index}:{primitive_index} is missing texture coordinates")]
    MissingTexCoords {
        mesh_index: usize,
        primitive_index: usize,
    },
    #[error("Primitive {mesh_index}:{primitive_index} has inconsistent attribute lengths: positions={positions_len}, normals={normals_len}, tex_coords={tex_coords_len}"
    )]
    InconsistentAttributeLengths {
        mesh_index: usize,
        primitive_index: usize,
        positions_len: usize,
        normals_len: usize,
        tex_coords_len: usize,
    },
    #[error("Invalid primitive {mesh_index}:{primitive_index} mode {mode:?}: only Points, Lines and Triangles are supported"
    )]
    InvalidPrimitiveMode {
        mesh_index: usize,
        primitive_index: usize,
        mode: Mode,
    },
    #[error("Cannot fit index into selected index type")]
    IndexOverflow,
}

#[derive(Clone, Debug)]
enum MaterialTextureType {
    BaseColor,
    Metallic,
    Roughness,
    Normal,
    Occlusion,
    Emissive,
}

impl MaterialTextureType {
    fn as_str(&self) -> &'static str {
        match self {
            MaterialTextureType::BaseColor => "base_color",
            MaterialTextureType::Metallic => "metallic",
            MaterialTextureType::Roughness => "roughness",
            MaterialTextureType::Normal => "normal",
            MaterialTextureType::Occlusion => "occlusion",
            MaterialTextureType::Emissive => "emissive",
        }
    }
}

fn texture_id(
    material_id: &AssetID,
    texture_type: MaterialTextureType,
    texture: &gltf::Texture,
) -> AssetID {
    AssetID::new(match texture.name() {
        None => format!(
            "{}_{}_{}_texture",
            material_id.as_str(),
            texture_type.as_str(),
            texture.index()
        ),
        Some(name) => format!(
            "{}_{}_{}",
            material_id.as_str(),
            texture_type.as_str(),
            name.to_string()
        ),
    })
}

fn material_id(
    mesh_id: &AssetID,
    mesh_index: usize,
    primitive_index: usize,
    material: &gltf::Material,
) -> AssetID {
    AssetID::new(match material.name() {
        None => format!(
            "{}_{}_{}_material",
            mesh_id.as_str(),
            mesh_index,
            primitive_index
        ),
        Some(name) => format!("{}_{}_{}", mesh_id.as_str(), mesh_index, name),
    })
}

fn process_texture(
    material_id: AssetID,
    texture_type: MaterialTextureType,
    texture: gltf::Texture,
    ctx: &ProcessCtx,
) -> Result<(AssetID, Vec<PartialIR>), MeshError> {
    let id = texture_id(&material_id, texture_type.clone(), &texture);
    let _measure = Measure::new(format!("Processed texture {}", id.as_str()));

    {
        let mut processed_textures = ctx.processed_textures.lock().unwrap();
        if let Some(id) = processed_textures.get(&texture.index()) {
            // Texture already processed. Just reuse it.
            return Ok((id.clone(), vec![]));
        } else {
            // Mark as processed to avoid duplicate processing in parallel threads.
            // We must do it when the mutex is locked, to avoid data races.
            processed_textures.insert(texture.index(), id.clone());
        }

        // Drop the lock before doing any heavy processing,
        // to avoid blocking other threads.
    }

    let data = &ctx.images.get(texture.source().index()).ok_or_else(|| {
        MeshError::MissingTextureSource {
            material_id: material_id.clone(),
            texture_type: texture_type.clone(),
            texture_index: texture.index(),
        }
    })?;

    Ok((
        id.clone(),
        vec![PartialIR {
            id: id.clone(),
            header: UserAssetHeader {
                asset_type: AssetType::Texture,
                dependencies: Default::default(),
                tags: vec![],
                author: Some("Auto-generated".to_string()),
                license: None,
            },
            ir: IRAsset::Texture(IRTexture {
                data: data.pixels.clone(),
                texture_type: IRTextureType::Texture2D {
                    width: data.width,
                    height: data.height,
                },
                pixel_format: match data.format {
                    Format::R8 => IRPixelFormat::R8,
                    Format::R8G8 => IRPixelFormat::R8G8,
                    Format::R8G8B8 => IRPixelFormat::R8G8B8,
                    Format::R8G8B8A8 => IRPixelFormat::R8G8B8A8,
                    Format::R16 => IRPixelFormat::R16,
                    Format::R16G16 => IRPixelFormat::R16G16,
                    Format::R16G16B16 => IRPixelFormat::R16G16B16,
                    Format::R16G16B16A16 => IRPixelFormat::R16G16B16A16,
                    Format::R32G32B32FLOAT => IRPixelFormat::R32G32B32FLOAT,
                    Format::R32G32B32A32FLOAT => IRPixelFormat::R32G32B32A32FLOAT,
                },
                ..Default::default()
            }),
        }],
    ))
}

fn process_material(
    mesh_index: usize,
    primitive_index: usize,
    material: gltf::Material,
    ctx: &ProcessCtx,
) -> Result<(AssetID, Vec<PartialIR>), MeshError> {
    let id = material_id(&ctx.mesh_id, mesh_index, primitive_index, &material);
    let _measure = Measure::new(format!("Processed material {}", id.as_str()));

    {
        let mut processed_materials = ctx.processed_materials.lock().unwrap();
        let index = material.index().unwrap();
        if let Some(id) = processed_materials.get(&index) {
            // Material already processed. Just reuse it.
            return Ok((id.clone(), vec![]));
        } else {
            // Mark as processed to avoid duplicate processing in parallel threads.
            // We must do it when the mutex is locked, to avoid
            processed_materials.insert(index, id.clone());
        }

        // Drop the lock before doing any heavy processing,
        // to avoid blocking other threads.
    }

    let mut irs = Vec::new();
    let mut dependencies = HashSet::new();
    let base_color_texture =
        if let Some(texture) = material.pbr_metallic_roughness().base_color_texture() {
            let (tex_id, generated) = process_texture(
                id.clone(),
                MaterialTextureType::BaseColor,
                texture.texture(),
                ctx,
            )?;
            dependencies.insert(tex_id.clone());
            irs.extend(generated);
            Some(tex_id)
        } else {
            None
        };
    let metallic_texture = if let Some(texture) = material
        .pbr_metallic_roughness()
        .metallic_roughness_texture()
    {
        let (tex_id, generated) = process_texture(
            id.clone(),
            MaterialTextureType::Metallic,
            texture.texture(),
            ctx,
        )?;
        dependencies.insert(tex_id.clone());
        irs.extend(generated);
        Some(tex_id)
    } else {
        None
    };
    let roughness_texture = if let Some(texture) = material
        .pbr_metallic_roughness()
        .metallic_roughness_texture()
    {
        let (tex_id, generated) = process_texture(
            id.clone(),
            MaterialTextureType::Roughness,
            texture.texture(),
            ctx,
        )?;
        dependencies.insert(tex_id.clone());
        irs.extend(generated);
        Some(tex_id)
    } else {
        None
    };

    irs.push(PartialIR {
        id: AssetID::from(id.clone()),
        header: UserAssetHeader {
            asset_type: AssetType::Material,
            dependencies,
            tags: vec![],
            author: Some("Auto-generated".to_string()),
            license: None,
        },
        ir: IRAsset::Material(IRMaterial {
            base_color_factor: material.pbr_metallic_roughness().base_color_factor(),
            base_color_texture,
            metallic_texture,
            metallic_factor: material.pbr_metallic_roughness().metallic_factor(),
            roughness_texture,
            roughness_factor: material.pbr_metallic_roughness().roughness_factor(),
            ..Default::default()
        }),
    });

    // TODO: Normal, occlusion and emissive textures
    Ok((AssetID::from(id), irs))
}

fn process_primitive(
    mesh_index: usize,
    primitive_index: usize,
    transform: Mat4,
    primitive: gltf::Primitive,
    ctx: &ProcessCtx,
) -> Result<PrimitiveProcessResult, MeshError> {
    let _measure = Measure::new(format!(
        "Processed primitive {}:{}",
        mesh_index, primitive_index
    ));
    let mut irs = Vec::new();
    let material = match primitive.material().index() {
        Some(_) => {
            let (id, generated) =
                process_material(mesh_index, primitive_index, primitive.material(), ctx)?;
            irs.extend(generated);
            Some(id)
        }
        None => None,
    };

    let reader = primitive.reader(|buffer| Some(&ctx.buffers[buffer.index()]));
    let indices_u32: Vec<_> = reader
        .read_indices()
        .ok_or(MeshError::MissingIndices {
            mesh_index,
            primitive_index,
        })?
        .into_u32()
        .collect();
    let mut indices = Vec::new();
    match ctx.index_type {
        IRIndexType::U16 => {
            indices.reserve(indices_u32.len() * 4);
            if indices_u32.len() > u16::MAX as usize {
                return Err(MeshError::IndexOverflow);
            }
            for i in indices_u32 {
                indices.extend_from_slice((i as u16).to_le_bytes().as_slice());
            }
        }
        IRIndexType::U32 => {
            indices.reserve(indices_u32.len() * 4);
            for i in indices_u32 {
                indices.extend_from_slice(i.to_le_bytes().as_slice());
            }
        }
    }

    let positions: Vec<_> = reader
        .read_positions()
        .ok_or(MeshError::MissingPositions {
            mesh_index,
            primitive_index,
        })?
        .map(|p| transform.transform_point3(glam::Vec3::from(p)))
        .collect();

    let normals: Vec<_> = if let Some(normals_iter) = reader.read_normals() {
        normals_iter
            .map(|n| transform.transform_vector3(glam::Vec3::from(n)).normalize())
            .collect()
    } else {
        Err(MeshError::MissingNormals {
            mesh_index,
            primitive_index,
        })?
    };

    let tex_coords: Vec<_> = if let Some(tex_coords_iter) = reader.read_tex_coords(0) {
        tex_coords_iter
            .into_f32()
            .map(|tc| glam::Vec2::from(tc))
            .collect()
    } else {
        Err(MeshError::MissingTexCoords {
            mesh_index,
            primitive_index,
        })?
    };

    // Check that all attributes have the same length
    if positions.len() != normals.len() || positions.len() != tex_coords.len() {
        return Err(MeshError::InconsistentAttributeLengths {
            mesh_index,
            primitive_index,
            positions_len: positions.len(),
            normals_len: normals.len(),
            tex_coords_len: tex_coords.len(),
        });
    }

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut vertices = Vec::with_capacity(positions.len() * size_of::<IRVertex>());
    for (position, (normal, tex_coord)) in
        positions.iter().zip(normals.iter().zip(tex_coords.iter()))
    {
        min = min.min(*position);
        max = max.max(*position);
        vertices.extend_from_slice(IRVertex::new(*position, *normal, *tex_coord).into_bytes());
    }

    Ok(PrimitiveProcessResult {
        irs,
        mesh: IRSubMesh {
            vertices,
            indices,
            material,
            bounds: IRMeshBounds {
                min: min.to_array(),
                max: max.to_array(),
            },
            topology: match primitive.mode() {
                Mode::Points => IRTopology::Points,
                Mode::Lines => IRTopology::Lines,
                Mode::Triangles => IRTopology::Triangles,
                _ => {
                    return Err(MeshError::InvalidPrimitiveMode {
                        mesh_index,
                        primitive_index,
                        mode: primitive.mode(),
                    })
                }
            },
        },
    })
}

impl<'a> MeshWrap<'a> {
    pub fn process(&self, index: usize) -> Result<Vec<PrimitiveProcessResult>, MeshError> {
        let _measure = Measure::new(format!("Processed mesh {}", index));
        self.node
            .primitives()
            .enumerate()
            .par_bridge()
            .map(|(i, primitive)| process_primitive(index, i, self.transform, primitive, &self.ctx))
            .collect()
    }
}

fn convert_mesh_inner(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserMeshAsset,
) -> Result<Vec<PartialIR>, MeshError> {
    // Load the GLTF file
    let path = user
        .source
        .as_path(cache_dir, cwd)
        .map_err(|e| MeshError::NotAFile(e.to_string()))?;
    let (document, buffers, images) = {
        let _measure = Measure::new(format!("Loaded mesh file '{}'", path.display()));
        gltf::import(path.clone()).map_err(|e| MeshError::LoadError(e.to_string()))?
    };

    if document.scenes().count() == 0 {
        return Err(MeshError::NoScene);
    }
    if document.scenes().count() > 1 {
        return Err(MeshError::MultipleScenes(document.scenes().count()));
    }

    // The name of the mesh is based on the file name
    let mesh_id = normalize_name(file.path.clone());
    let ctx = ProcessCtx {
        buffers: &buffers,
        index_type: IRIndexType::U32,
        images: &images,
        // Dependencies of the mesh.
        // This is shared between threads to avoid generating the same material multiple times.
        processed_materials: Arc::new(Mutex::new(HashMap::new())),
        processed_textures: Arc::new(Mutex::new(Default::default())),
        mesh_id: mesh_id.clone(),
    };

    let scene = document.scenes().next().unwrap();

    // Collect all nodes as a flat list of meshes with their transforms
    // Since IR is not hierarchical, we need to flatten the scene graph.
    // This is done in a single thread, since the scene graph is usually not very large.
    // The actual mesh processing is done in parallel later.
    fn collect_nodes<'a>(
        node: gltf::Node<'a>,
        parent_transform: Mat4,
        nodes: &mut Vec<MeshWrap<'a>>,
        ctx: ProcessCtx<'a>,
    ) {
        // Compute transform of the current node
        let transform = parent_transform * transform_to_matrix(node.transform());
        if let Some(mesh) = node.mesh() {
            nodes.push(MeshWrap {
                transform,
                node: mesh.clone(),
                ctx: ctx.clone(),
            });
        }

        for child in node.children() {
            collect_nodes(child, transform, nodes, ctx.clone());
        }
    }

    let mut meshes = Vec::new();
    for node in scene.nodes() {
        collect_nodes(node, Mat4::IDENTITY, &mut meshes, ctx.clone());
    }

    // Process each mesh in parallel. The final order is not important.
    let results = meshes
        .par_iter()
        .enumerate()
        .map(|(i, mesh)| mesh.process(i))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    // Add dependencies of the generated materials
    // Textures are already added as dependencies of the materials.
    let mut header = file.asset.header.clone();
    for id in ctx.processed_materials.lock().unwrap().values() {
        header.dependencies.insert(id.clone());
    }

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
            index_type: ctx.index_type,
        }),
        header,
        mesh_id.clone(),
    ));

    Ok(irs)
}

pub fn convert_mesh(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserMeshAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    convert_mesh_inner(file, cache_dir, cwd, user).map_err(|e| anyhow::anyhow!(e))
}
