use crate::AssetID;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::mem::offset_of;

// pub const IR_MAX_BONE_INFLUENCES: usize = 4;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IRMeshBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl IRMeshBounds {
    pub fn min(&self) -> Vec3 {
        Vec3::from(self.min)
    }

    pub fn max(&self) -> Vec3 {
        Vec3::from(self.max)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[repr(C)]
#[repr(packed)]
pub struct IRVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    // pub tangent: [f32; 3],
    // pub bitangent: [f32; 3],
    // pub bone_indices: [u32; IR_MAX_BONE_INFLUENCES],
    // pub bone_weights: [f32; IR_MAX_BONE_INFLUENCES],
}

#[allow(dead_code)]
impl IRVertex {
    pub fn layout() -> [IRMeshLayout; 3] {
        [
            IRMeshLayout {
                field: IRMeshField::Position,
                sample_type: IRMeshLayoutSampleType::Float,
                samples: 3, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, position),
            },
            IRMeshLayout {
                field: IRMeshField::Normal,
                sample_type: IRMeshLayoutSampleType::Float,
                samples: 3, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, normal),
            },
            IRMeshLayout {
                field: IRMeshField::TexCoord,
                sample_type: IRMeshLayoutSampleType::Float,
                samples: 2, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, tex_coord),
            },
        ]
    }

    pub fn position(&self) -> Vec3 {
        Vec3::from(self.position)
    }

    pub fn normal(&self) -> Vec3 {
        Vec3::from(self.normal)
    }

    pub fn tex_coord(&self) -> Vec2 {
        Vec2::from(self.tex_coord)
    }

    pub fn into_bytes<'a>(self) -> &'a [u8] {
        unsafe {
            std::slice::from_raw_parts(
                (&self as *const IRVertex) as *const u8,
                size_of::<IRVertex>(),
            )
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRPrimitive {
    Points,
    Lines,
    LineLoop,
    LineStrip,
    Triangles,
    TriangleStrip,
    TriangleFan,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IRSubMesh {
    // Raw bytes of vertices
    // (should be multiple of size_of::<IRVertex>())
    pub vertices: Vec<u8>,
    pub indices: Vec<u32>,
    pub material: AssetID,
    pub bounds: IRMeshBounds,
    pub primitive: IRPrimitive,
    pub primitives_count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IRMesh {
    pub submesh: Vec<IRSubMesh>,
    pub bounds: IRMeshBounds,
}

impl IRSubMesh {
    pub fn raw_vertices<'a>(&self) -> &'a [u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.vertices.as_ptr(),
                self.vertices.len() * size_of::<IRVertex>(),
            )
        }
    }

    pub fn raw_indices<'a>(&self) -> &'a [u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.indices.as_ptr() as *const u8,
                self.indices.len() * size_of::<u32>(),
            )
        }
    }
}

impl Debug for IRSubMesh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IRMesh")
            .field("vertices_count", &self.vertices.len())
            .field("indices_count", &self.indices.len())
            .field("material", &self.material)
            .field("bounds", &self.bounds)
            .field("primitive", &self.primitive)
            .field("primitives_count", &self.primitives_count)
            .finish()
    }
}

impl Default for IRSubMesh {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            material: AssetID::default(),
            bounds: IRMeshBounds {
                min: [0.0, 0.0, 0.0],
                max: [0.0, 0.0, 0.0],
            },
            primitive: IRPrimitive::Points,
            primitives_count: 0,
        }
    }
}

pub enum IRMeshField {
    Position,
    Normal,
    Tangent,
    Bitangent,
    TexCoord,
    BoneIndices,
    BoneWeights,
}

pub enum IRMeshLayoutSampleType {
    Float,
    U32,
}

pub struct IRMeshLayout {
    pub field: IRMeshField,
    pub sample_type: IRMeshLayoutSampleType,
    pub samples: usize,
    pub stride_bytes: usize,
    pub offset_bytes: usize,
}

impl IRMesh {
    pub fn memory_usage(&self) -> usize {
        let mut sum = size_of::<IRMesh>();
        sum += self.submesh.capacity() * size_of::<IRSubMesh>();
        for submesh in &self.submesh {
            sum += submesh.vertices.capacity() * size_of::<IRVertex>();
            sum += submesh.indices.capacity() * size_of::<u32>();
        }
        sum
    }
}
