use crate::ir::{normalize_name, PartialIR};
use crate::user::{UserAssetHeader, UserMeshAsset};
use crate::UserAssetFile;
use dawn_assets::ir::material::IRMaterial;
use dawn_assets::ir::mesh::{
    IRIndexType, IRMesh, IRMeshBounds, IRMeshVertex, IRSubMesh, IRTopology,
};
use dawn_assets::ir::texture::{IRPixelFormat, IRTexture, IRTextureType};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetID, AssetType};
use dawn_util::profile::Measure;
use glam::{vec3, Mat4, Vec2, Vec3, Vec4};
use gltf::buffer::Data;
use gltf::image::Format;
use gltf::mesh::Mode;
use gltf::scene::Transform;
use gltf::Texture;
use image::DynamicImage;
use log::warn;
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
    processed_named_textures: Arc<Mutex<HashMap<usize, AssetID>>>,
    used_common_textures: Arc<&'a Mutex<HashSet<AssetID>>>,
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
    #[error("Material {material_id} is missing texture source at texture index {texture_index}")]
    MissingTextureSource {
        material_id: AssetID,
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
    #[error("Unexpected texture format of material {texture_id} at index {index}. Expected {expected:?}, found {found:?}"
    )]
    UnexpectedTextureFormat {
        texture_id: AssetID,
        index: usize,
        expected: Vec<Format>,
        found: Format,
    },
}

#[derive(Clone, Debug)]
enum MaterialTexture<'a> {
    Albedo {
        texture: Option<Texture<'a>>,
        fallback_color: Vec4,
    },
    MetallicRoughness {
        texture: Option<Texture<'a>>,
        fallback_value: (f32, f32),
    },
    Normal {
        texture: Option<Texture<'a>>,
        multiplier: f32,
    },
    Occlusion {
        texture: Option<Texture<'a>>,
        multiplier: f32,
    },
}

impl<'a> MaterialTexture<'a> {
    fn as_multipler(&self) -> f32 {
        match self {
            MaterialTexture::Albedo { .. } => 1.0,
            MaterialTexture::MetallicRoughness { .. } => 1.0,
            MaterialTexture::Normal { multiplier, .. } => *multiplier,
            MaterialTexture::Occlusion { multiplier, .. } => *multiplier,
        }
    }

    fn as_texture(&self) -> Option<&Texture> {
        match self {
            MaterialTexture::Albedo { texture, .. } => texture.as_ref(),
            MaterialTexture::MetallicRoughness { texture, .. } => texture.as_ref(),
            MaterialTexture::Normal { texture, .. } => texture.as_ref(),
            MaterialTexture::Occlusion { texture, .. } => texture.as_ref(),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            MaterialTexture::Albedo { .. } => "albedo",
            MaterialTexture::MetallicRoughness { .. } => "metallic_roughness",
            MaterialTexture::Normal { .. } => "normal",
            MaterialTexture::Occlusion { .. } => "occlusion",
        }
    }

    fn default_format(&self) -> Format {
        match self {
            MaterialTexture::Albedo { .. } => Format::R8G8B8,
            MaterialTexture::MetallicRoughness { .. } => Format::R8G8,
            MaterialTexture::Normal { .. } => Format::R8G8B8,
            MaterialTexture::Occlusion { .. } => Format::R8,
        }
    }

    fn allowed_formats(&self) -> &[Format] {
        match self {
            MaterialTexture::Albedo { .. } => &[Format::R8G8B8, Format::R8G8B8A8],
            MaterialTexture::MetallicRoughness { .. } => &[Format::R8G8B8],
            MaterialTexture::Normal { .. } => &[Format::R8G8B8],
            MaterialTexture::Occlusion { .. } => &[Format::R8],
        }
    }
}

fn texture_named_id(
    material_id: &AssetID,
    texture_type: &MaterialTexture,
    name: Option<&str>,
) -> AssetID {
    AssetID::new(match name {
        None => format!("{}_{}_texture", material_id.as_str(), texture_type.as_str(),),
        Some(name) => format!(
            "{}_{}_{}",
            material_id.as_str(),
            texture_type.as_str(),
            name.to_string()
        ),
    })
}

fn texture_unnamed_r_texture_id(value: f32) -> AssetID {
    AssetID::new(format!("common_r_texture_{:03}", (value * 1000.0) as u32))
}

fn texture_unnamed_rg_texture_id(color: Vec2) -> AssetID {
    AssetID::new(format!(
        "common_rg_texture_{:03}_{:03}",
        (color.x * 100.0) as u32,
        (color.y * 100.0) as u32
    ))
}

fn texture_unnamed_rgb_texture_id(color: Vec3) -> AssetID {
    AssetID::new(format!(
        "common_rgb_texture_{:03}_{:03}_{:03}",
        (color.x * 100.0) as u32,
        (color.y * 100.0) as u32,
        (color.z * 100.0) as u32
    ))
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

fn fake_texture_rgb(id: AssetID, color: Vec3) -> Result<(AssetID, Vec<PartialIR>), MeshError> {
    let data = vec![
        (color.x * 255.0) as u8,
        (color.y * 255.0) as u8,
        (color.z * 255.0) as u8,
        255u8,
    ];
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
                data,
                texture_type: IRTextureType::Texture2D {
                    width: 1,
                    height: 1,
                },
                pixel_format: IRPixelFormat::RGBA8,
                ..Default::default()
            }),
        }],
    ))
}


fn fake_texture_r(id: AssetID, value: f32) -> Result<(AssetID, Vec<PartialIR>), MeshError> {
    let data = vec![(value * 255.0) as u8];
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
                data,
                texture_type: IRTextureType::Texture2D {
                    width: 1,
                    height: 1,
                },
                pixel_format: IRPixelFormat::R8,
                ..Default::default()
            }),
        }],
    ))
}

fn try_resample(from: Format, to: Format, data: &Vec<u8>) -> Option<Vec<u8>> {
    let mut result = Vec::with_capacity(data.len());
    match (from, to) {
        (Format::R8G8B8A8, Format::R8) => {
            // Convert RGBA8 to R8 by taking the red channel
            for chunk in data.chunks(4) {
                result.push(chunk[0]);
            }
        }
        (Format::R8G8B8, Format::R8) => {
            // Convert RGB8 to R8 by taking the red channel
            for chunk in data.chunks(3) {
                result.push(chunk[0]);
            }
        }
        _ => {
            return None;
            // Unsupported conversion
        }
    };

    Some(result)
}

fn process_texture(
    material_id: AssetID,
    texture_type: MaterialTexture,
    ctx: &ProcessCtx,
) -> Result<(AssetID, Vec<PartialIR>), MeshError> {
    if let Some(texture) = texture_type.as_texture() {
        let id = texture_named_id(&material_id, &texture_type, texture.name());
        let _measure = Measure::new(format!("Processed texture {}", id.as_str()));

        {
            let mut processed_textures = ctx.processed_named_textures.lock().unwrap();

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
                texture_index: texture.index(),
            }
        })?;
        let mut format = data.format;
        let width = data.width;
        let height = data.height;
        let mut data = &data.pixels;

        let mut resampled: Option<Vec<u8>> = None;
        let expected_formats = texture_type.allowed_formats();
        if !expected_formats.contains(&format) {
            if let Some(result) = try_resample(format, texture_type.default_format(), data) {
                warn!(
                    "Resampled texture {} from format {:?} to {:?}",
                    id.as_str(),
                    format,
                    texture_type.default_format()
                );
                format = texture_type.default_format();
                resampled = Some(result);
                data = resampled.as_ref().unwrap();
            } else {
                return Err(MeshError::UnexpectedTextureFormat {
                    texture_id: id.clone(),
                    index: texture.index(),
                    expected: texture_type.allowed_formats().to_vec(),
                    found: format,
                });
            }
        }

        // TODO: Handle the multiplier for normal and occlusion maps.

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
                    data: data.clone(),
                    texture_type: IRTextureType::Texture2D { width, height },
                    pixel_format: match format {
                        Format::R8 => IRPixelFormat::R8,
                        Format::R8G8 => IRPixelFormat::RG8,
                        Format::R8G8B8 => IRPixelFormat::RGB8,
                        Format::R8G8B8A8 => IRPixelFormat::RGBA8,
                        Format::R16 => IRPixelFormat::R16,
                        Format::R16G16 => IRPixelFormat::RG16,
                        Format::R16G16B16 => IRPixelFormat::RGB16,
                        Format::R16G16B16A16 => IRPixelFormat::RGBA16,
                        Format::R32G32B32FLOAT => IRPixelFormat::RGB32F,
                        Format::R32G32B32A32FLOAT => IRPixelFormat::RGBA32F,
                    },
                    ..Default::default()
                }),
            }],
        ))
    } else {
        // No texture. Create a fake 1x1 texture with the fallback color/value.
        const FALLBACK_OCCLUSION: f32 = 1.0;
        const FALLBACK_NORMAL: Vec3 = vec3(0.0, 0.0, 0.0);
        let id = match texture_type {
            MaterialTexture::Albedo { .. } => texture_named_id(&material_id, &texture_type, None),
            MaterialTexture::MetallicRoughness { .. } => {
                texture_named_id(&material_id, &texture_type, None)
            }
            MaterialTexture::Normal { .. } => texture_unnamed_rgb_texture_id(FALLBACK_NORMAL),
            MaterialTexture::Occlusion { .. } => texture_unnamed_r_texture_id(FALLBACK_OCCLUSION),
        };

        {
            let mut used_common_textures = ctx.used_common_textures.lock().unwrap();

            if used_common_textures.contains(&id) {
                // Common texture already processed. Just reuse it.
                return Ok((id.clone(), vec![]));
            } else {
                // Mark as used to avoid duplicate processing in parallel threads.
                // We must do it when the mutex is locked, to avoid data
                used_common_textures.insert(id.clone());
            }
        }

        match texture_type {
            MaterialTexture::Albedo { fallback_color, .. } => {
                fake_texture_rgb(id, fallback_color.truncate())
            }
            MaterialTexture::MetallicRoughness { fallback_value, .. } => {
                // Pack metallic (R) and roughness (G) into a 2-channel texture
                let color = vec3(fallback_value.0, fallback_value.1, 0.0);
                fake_texture_rgb(id, color)
            }
            MaterialTexture::Normal { .. } => fake_texture_rgb(id, FALLBACK_NORMAL),
            MaterialTexture::Occlusion { .. } => fake_texture_r(id, FALLBACK_OCCLUSION),
        }
    }
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

    let (albedo_id, albedo_irs) = process_texture(
        id.clone(),
        if let Some(pbr) = material.pbr_metallic_roughness().base_color_texture() {
            MaterialTexture::Albedo {
                texture: Some(pbr.texture()),
                fallback_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            }
        } else {
            MaterialTexture::Albedo {
                texture: None,
                fallback_color: material.pbr_metallic_roughness().base_color_factor().into(),
            }
        },
        ctx,
    )?;

    let (metallic_roughness_id, metallic_roughness_irs) = process_texture(
        id.clone(),
        if let Some(pbr) = material
            .pbr_metallic_roughness()
            .metallic_roughness_texture()
        {
            MaterialTexture::MetallicRoughness {
                texture: Some(pbr.texture()),
                fallback_value: (
                    material.pbr_metallic_roughness().metallic_factor(),
                    material.pbr_metallic_roughness().roughness_factor(),
                ),
            }
        } else {
            MaterialTexture::MetallicRoughness {
                texture: None,
                fallback_value: (
                    material.pbr_metallic_roughness().metallic_factor(),
                    material.pbr_metallic_roughness().roughness_factor(),
                ),
            }
        },
        ctx,
    )?;

    let (normal_id, normal_irs) = process_texture(
        id.clone(),
        if let Some(normal) = material.normal_texture() {
            MaterialTexture::Normal {
                texture: Some(normal.texture()),
                multiplier: normal.scale(),
            }
        } else {
            MaterialTexture::Normal {
                texture: None,
                multiplier: 0.0,
            }
        },
        ctx,
    )?;

    let (occlusion_id, occlusion_irs) = process_texture(
        id.clone(),
        if let Some(occlusion) = material.occlusion_texture() {
            MaterialTexture::Occlusion {
                texture: Some(occlusion.texture()),
                multiplier: occlusion.strength(),
            }
        } else {
            MaterialTexture::Occlusion {
                texture: None,
                multiplier: 0.0,
            }
        },
        ctx,
    )?;

    let mut dependencies = HashSet::new();
    dependencies.insert(albedo_id.clone());
    dependencies.insert(metallic_roughness_id.clone());
    dependencies.insert(normal_id.clone());
    dependencies.insert(occlusion_id.clone());

    let mut irs = Vec::new();
    irs.extend(albedo_irs);
    irs.extend(metallic_roughness_irs);
    irs.extend(normal_irs);
    irs.extend(occlusion_irs);

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
            albedo: albedo_id,
            normal: normal_id,
            metallic_roughness: metallic_roughness_id,
            occlusion: occlusion_id,
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

    // Try to get tangent and bitangent, if not present, compute them
    // For simplicity, we will set them to zero for now.
    let mut tangents = vec![Vec3::ZERO; positions.len()];
    let mut bitangents = vec![Vec3::ZERO; positions.len()];
    if let Some(tangents_iter) = reader.read_tangents() {
        for (i, t) in tangents_iter.enumerate() {
            let t = Vec3::from([t[0], t[1], t[2]]).normalize();
            tangents[i] = t;
        }
    } else {
        // TODO: Compute tangents from normals and texture coordinates
    }
    // Calculate bitangents from normals and tangents
    for i in 0..positions.len() {
        let n = normals[i];
        let t = tangents[i];
        let b = n.cross(t).normalize();
        bitangents[i] = b;
    }

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    let mut vertices = Vec::with_capacity(positions.len() * size_of::<IRMeshVertex>());
    for (((((position, normal), tex_coord), tangent), bitangent)) in positions
        .iter()
        .zip(normals.iter())
        .zip(tex_coords.iter())
        .zip(tangents.iter())
        .zip(bitangents.iter())
    {
        min = min.min(*position);
        max = max.max(*position);
        vertices.extend_from_slice(
            IRMeshVertex::new(*position, *normal, *tex_coord, *tangent, *bitangent).into_bytes(),
        );
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

    static USED_COMMON_TEXTURES: once_cell::sync::Lazy<Mutex<HashSet<AssetID>>> =
        once_cell::sync::Lazy::new(|| Mutex::new(HashSet::new()));

    // The name of the mesh is based on the file name
    let mesh_id = normalize_name(file.path.clone());
    let ctx = ProcessCtx {
        buffers: &buffers,
        index_type: IRIndexType::U32,
        images: &images,
        // Dependencies of the mesh.
        // This is shared between threads to avoid generating the same material multiple times.
        processed_materials: Arc::new(Mutex::new(HashMap::new())),
        processed_named_textures: Arc::new(Mutex::new(HashMap::new())),
        used_common_textures: Arc::new(&USED_COMMON_TEXTURES),
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

    for ir in irs.iter() {
        match &ir.ir {
            IRAsset::Texture(tex) => match tex.pixel_format {
                IRPixelFormat::R8 => {
                    let img = DynamicImage::ImageLuma8(
                        image::ImageBuffer::from_raw(
                            match tex.texture_type {
                                IRTextureType::Texture2D { width, height } => width,
                                _ => 1,
                            },
                            match tex.texture_type {
                                IRTextureType::Texture2D { width: _, height } => height,
                                _ => 1,
                            },
                            tex.data.clone(),
                        )
                        .unwrap(),
                    );
                    let mut img = img.to_luma8();
                    img.copy_from_slice(&tex.data);
                    img.save(cache_dir.join(format!("{}.png", ir.id.as_str())))
                        .unwrap();
                }
                IRPixelFormat::RGB8 => {
                    let img = DynamicImage::ImageRgb8(
                        image::ImageBuffer::from_raw(
                            match tex.texture_type {
                                IRTextureType::Texture2D { width, height } => width,
                                _ => 1,
                            },
                            match tex.texture_type {
                                IRTextureType::Texture2D { width: _, height } => height,
                                _ => 1,
                            },
                            tex.data.clone(),
                        )
                        .unwrap(),
                    );
                    let mut img = img.to_rgb8();
                    img.copy_from_slice(&tex.data);
                    img.save(cache_dir.join(format!("{}.png", ir.id.as_str())))
                        .unwrap();
                }
                IRPixelFormat::RGBA8 => {
                    let img = DynamicImage::ImageRgba8(
                        image::ImageBuffer::from_raw(
                            match tex.texture_type {
                                IRTextureType::Texture2D { width, height } => width,
                                _ => 1,
                            },
                            match tex.texture_type {
                                IRTextureType::Texture2D { width: _, height } => height,
                                _ => 1,
                            },
                            tex.data.clone(),
                        )
                        .unwrap(),
                    );
                    let mut img = img.to_rgba8();
                    img.copy_from_slice(&tex.data);
                    img.save(cache_dir.join(format!("{}.png", ir.id.as_str())))
                        .unwrap();
                }
                _ => {
                    panic!(
                        "Unsupported texture format for saving: {:?}",
                        tex.pixel_format
                    );
                }
            },
            _ => {}
        }
    }

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
