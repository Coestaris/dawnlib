use crate::ir::{normalize_name, PartialIR};
use crate::user::{UserAssetHeader, UserMeshAsset};
use crate::UserAssetFile;
use dawn_assets::ir::material::IRMaterial;
use dawn_assets::ir::mesh::{IRMesh, IRMeshBounds, IRPrimitive, IRSubMesh, IRVertex};
use dawn_assets::ir::texture::{IRPixelDataType, IRPixelFormat, IRTexture, IRTextureType};
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

pub fn process_texture(
    material_id: AssetID,
    texture_type: usize,
    texture: gltf::Texture,
    ctx: &ProcessCtx,
) -> Result<(AssetID, Vec<PartialIR>), String> {
    let _measure = Measure::new(format!(
        "Processed texture {}:{}",
        material_id.as_str(),
        texture_type
    ));

    {
        // let mut processed_textures = ctx.processed_textures.lock().unwrap();
        // if let Some(id) = processed_textures.get(&texture.index()) {
        //     // Texture already processed. Just reuse it.
        //     return Ok((id.clone(), vec![]));
        // }
    }

    let id = AssetID::new(match texture.name() {
        None => format!(
            "{}_{}_{}_texture",
            material_id.as_str(),
            texture_type,
            texture.index()
        ),
        Some(name) => format!(
            "{}_{}_{}",
            material_id.as_str(),
            texture_type,
            name.to_string()
        ),
    });

    let data = &ctx.images.get(texture.source().index()).ok_or(format!(
        "Texture {}:{} has invalid image source index {}",
        material_id.as_str(),
        texture_type,
        texture.source().index()
    ))?;

    let mut irs = Vec::new();
    irs.push(PartialIR {
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
                Format::R8G8 => IRPixelFormat::RG8,
                Format::R8G8B8 => IRPixelFormat::RGB(IRPixelDataType::U8),
                Format::R8G8B8A8 => IRPixelFormat::RGBA(IRPixelDataType::U8),
                Format::R16 => IRPixelFormat::R16,
                Format::R16G16 => IRPixelFormat::RG16,
                Format::R16G16B16 => IRPixelFormat::RGB(IRPixelDataType::U16),
                Format::R16G16B16A16 => IRPixelFormat::RGBA(IRPixelDataType::U16),
                Format::R32G32B32FLOAT => IRPixelFormat::RGB(IRPixelDataType::F32),
                Format::R32G32B32A32FLOAT => IRPixelFormat::RGBA(IRPixelDataType::F32),
            },
            ..Default::default()
        }),
    });

    {
        let mut processed_textures = ctx.processed_textures.lock().unwrap();
        processed_textures.insert(texture.index(), id.clone());
    }

    Ok((id, irs))
}

pub fn process_material(
    mesh_index: usize,
    primitive_index: usize,
    material: gltf::Material,
    ctx: &ProcessCtx,
) -> Result<(AssetID, Vec<PartialIR>), String> {
    let _measure = Measure::new(format!(
        "Processed material {}:{}",
        mesh_index, primitive_index
    ));
    let id = AssetID::new(match material.name() {
        None => format!(
            "{}_{}_{}_material",
            ctx.mesh_id.as_str(),
            mesh_index,
            primitive_index
        ),
        Some(name) => format!("{}_{}_{}", ctx.mesh_id.as_str(), mesh_index, name),
    });

    let mut irs = Vec::new();
    let mut dependencies = HashSet::new();
    let base_color_texture =
        if let Some(texture) = material.pbr_metallic_roughness().base_color_texture() {
            let (tex_id, mut generated) = process_texture(id.clone(), 0, texture.texture(), ctx)?;
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
        let (tex_id, mut generated) = process_texture(id.clone(), 1, texture.texture(), ctx)?;
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
        let (tex_id, mut generated) = process_texture(id.clone(), 2, texture.texture(), ctx)?;
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

pub fn process_primitive(
    mesh_index: usize,
    primitive_index: usize,
    transform: Mat4,
    primitive: gltf::Primitive,
    ctx: &ProcessCtx,
) -> Result<PrimitiveProcessResult, String> {
    let _measure = Measure::new(format!(
        "Processed primitive {}:{}",
        mesh_index, primitive_index
    ));
    let mut irs = Vec::new();
    let material = match primitive.material().index() {
        Some(index) => {
            let id = { ctx.processed_materials.lock().unwrap().get(&index).cloned() };
            if let Some(id) = id {
                // Material already processed. Just reuse it.
                Some(id.clone())
            } else {
                // Process the material
                let (id, mut generated) =
                    process_material(mesh_index, primitive_index, primitive.material(), ctx)?;
                ctx.processed_materials
                    .lock()
                    .unwrap()
                    .insert(index, id.clone());
                irs.extend(generated);
                Some(id)
            }
        }
        None => None,
    };

    let reader = primitive.reader(|buffer| Some(&ctx.buffers[buffer.index()]));
    let indices: Vec<_> = reader
        .read_indices()
        .ok_or(format!(
            "Primitive {}:{} is missing indices",
            mesh_index, primitive_index
        ))?
        .into_u32()
        .collect();

    let mut positions: Vec<_> = reader
        .read_positions()
        .ok_or(format!(
            "Primitive {}:{} is missing positions",
            mesh_index, primitive_index
        ))?
        .map(|p| transform.transform_point3(glam::Vec3::from(p)))
        .collect();

    let mut normals: Vec<_> = if let Some(normals_iter) = reader.read_normals() {
        normals_iter
            .map(|n| transform.transform_vector3(glam::Vec3::from(n)).normalize())
            .collect()
    } else {
        Err(format!(
            "Primitive {}:{} is missing normals",
            mesh_index, primitive_index
        ))?
    };

    let mut tex_coords: Vec<_> = if let Some(tex_coords_iter) = reader.read_tex_coords(0) {
        tex_coords_iter
            .into_f32()
            .map(|tc| glam::Vec2::from(tc))
            .collect()
    } else {
        Err(format!(
            "Primitive {}:{} is missing texture coordinates",
            mesh_index, primitive_index
        ))?
    };

    // Check that all attributes have the same length
    if positions.len() != normals.len() || positions.len() != tex_coords.len() {
        return Err(format!(
            "Primitive {}:{} has inconsistent attribute lengths: positions={}, normals={}, tex_coords={}",
            mesh_index, primitive_index, positions.len(), normals.len(), tex_coords.len()
        ));
    }

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut data = Vec::with_capacity(positions.len() * size_of::<IRVertex>());
    for (position, (normal, tex_coord)) in
        positions.iter().zip(normals.iter().zip(tex_coords.iter()))
    {
        min = min.min(*position);
        max = max.max(*position);
        data.extend_from_slice(IRVertex::new(*position, *normal, *tex_coord).into_bytes());
    }

    let (primitive, primitives_count) = match primitive.mode() {
        Mode::Points => (IRPrimitive::Points, indices.len()),
        Mode::Lines => (IRPrimitive::Lines, indices.len() / 2),
        Mode::Triangles => (IRPrimitive::Triangles, indices.len() / 3),
        _ => {
            return Err(format!(
                "Primitive {}:{} has unsupported mode {:?}. Only Points, Lines and Triangles are supported.",
                mesh_index, primitive_index, primitive.mode()
            ));
        }
    };

    Ok(PrimitiveProcessResult {
        irs,
        mesh: IRSubMesh {
            vertices: data,
            indices,
            material,
            bounds: IRMeshBounds {
                min: min.to_array(),
                max: max.to_array(),
            },
            primitive,
            primitives_count,
        },
    })
}

impl<'a> MeshWrap<'a> {
    pub fn process(&self, index: usize) -> Result<Vec<PrimitiveProcessResult>, String> {
        let _measure = Measure::new(format!("Processed mesh {}", index));
        self.node
            .primitives()
            .enumerate()
            .par_bridge()
            .map(|(i, primitive)| process_primitive(index, i, self.transform, primitive, &self.ctx))
            .collect()
    }
}

pub fn convert_mesh(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserMeshAsset,
) -> Result<Vec<PartialIR>, String> {
    // Load the GLTF file
    let path = user.source.as_path(cache_dir, cwd)?;
    let (document, buffers, images) = {
        let _measure = Measure::new(format!("Loaded mesh file '{}'", path.display()));
        gltf::import(path.clone()).map_err(|e| {
            format!(
                "Failed to load mesh GLTF (or GLB) file '{}': {}",
                path.display(),
                e
            )
        })?
    };

    if document.scenes().count() == 0 {
        return Err(format!(
            "The mesh file '{}' does not contain any scene.",
            path.display()
        ));
    }
    if document.scenes().count() > 1 {
        return Err(format!(
            "The mesh file '{}' contains multiple scenes. Only one scene is supported.",
            path.display()
        ));
    }

    // The name of the mesh is based on the file name
    let mesh_id = normalize_name(file.path.clone());
    let ctx = ProcessCtx {
        buffers: &buffers,
        images: &images,
        // Dependencies of the mesh.
        // This is shared between threads to avoid generating the same material multiple times.
        processed_materials: Arc::new(Mutex::new(HashMap::new())),
        processed_textures: Arc::new(Mutex::new(Default::default())),
        mesh_id: mesh_id.clone(),
    };

    let scene = document.scenes().next().unwrap();

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
        }),
        header,
        mesh_id.clone(),
    ));

    Ok(irs)
}
