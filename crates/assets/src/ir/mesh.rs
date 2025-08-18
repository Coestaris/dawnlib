use crate::AssetID;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
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

    fn position(&self) -> Vec3 {
        Vec3::from(self.position)
    }

    fn normal(&self) -> Vec3 {
        Vec3::from(self.normal)
    }

    fn tex_coord(&self) -> Vec2 {
        Vec2::from(self.tex_coord)
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IRMesh {
    // All the geometry hierarchy is baked into the single World-Space mesh.
    pub vertices: Vec<IRVertex>,
    pub indices: Vec<u32>,
    pub material: AssetID,
    pub bounds: IRMeshBounds,
    pub primitive: IRPrimitive,
    pub primitives_count: usize,
}

impl IRMesh {
    pub fn raw_vertices<'a>(&self) -> &'a [u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.vertices.as_ptr() as *const u8,
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

impl Default for IRMesh {
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
