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
    pub fn new(pos: Vec3, norm: Vec3, tex: Vec2) -> Self {
        Self {
            position: pos.to_array(),
            normal: norm.to_array(),
            tex_coord: tex.to_array(),
        }
    }

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
pub enum IRTopology {
    Points,
    Lines,
    Triangles,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum IRIndexType {
    U16,
    U32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IRSubMesh {
    // Raw bytes of vertices
    // (should be multiple of size_of::<IRVertex>())
    #[serde(with = "serde_bytes")]
    pub vertices: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub indices: Vec<u8>,
    pub material: Option<AssetID>,
    pub bounds: IRMeshBounds,
    pub topology: IRTopology,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IRMesh {
    pub submesh: Vec<IRSubMesh>,
    pub bounds: IRMeshBounds,
    pub index_type: IRIndexType,
}

impl IRSubMesh {
    pub fn raw_vertices(&self) -> &[u8] {
        &self.vertices
    }

    pub fn raw_indices(&self) -> &[u8] {
        &self.indices
    }
}

impl Debug for IRSubMesh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IRMesh")
            .field("vertices_count", &self.vertices.len())
            .field("indices_count", &self.indices.len())
            .field("material", &self.material)
            .field("bounds", &self.bounds)
            .field("topology", &self.topology)
            .finish()
    }
}

impl Default for IRSubMesh {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            material: None,
            bounds: IRMeshBounds {
                min: [0.0, 0.0, 0.0],
                max: [0.0, 0.0, 0.0],
            },
            topology: IRTopology::Points,
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
