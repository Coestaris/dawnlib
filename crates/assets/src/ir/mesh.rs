use crate::AssetID;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

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

pub const fn layout_of_submesh(tangent_valid: bool, skinning: bool) -> &'static [IRMeshLayoutItem] {
    const BASE: [IRMeshLayoutItem; 3] = [
        IRMeshLayoutItem {
            field: IRLayoutField::Position,
            sample_type: IRLayoutSampleType::Float,
            samples: 3, // floats
            stride_bytes: 28,
            offset_bytes: 0,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::Normal,
            sample_type: IRLayoutSampleType::Float,
            samples: 3, // floats
            stride_bytes: 28,
            offset_bytes: 12,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::TexCoord,
            sample_type: IRLayoutSampleType::Float,
            samples: 2, // floats
            stride_bytes: 28,
            offset_bytes: 24,
        },
    ];

    const TANGENT: [IRMeshLayoutItem; 5] = [
        IRMeshLayoutItem {
            field: IRLayoutField::Position,
            sample_type: IRLayoutSampleType::Float,
            samples: 3, // floats
            stride_bytes: 56,
            offset_bytes: 0,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::Normal,
            sample_type: IRLayoutSampleType::Float,
            samples: 3, // floats
            stride_bytes: 56,
            offset_bytes: 12,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::TexCoord,
            sample_type: IRLayoutSampleType::Float,
            samples: 2, // floats
            stride_bytes: 56,
            offset_bytes: 24,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::Tangent,
            sample_type: IRLayoutSampleType::Float,
            samples: 3, // floats
            stride_bytes: 56,
            offset_bytes: 32,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::Bitangent,
            sample_type: IRLayoutSampleType::Float,
            samples: 3, // floats
            stride_bytes: 56,
            offset_bytes: 44,
        },
    ];

    const SKINNING: [IRMeshLayoutItem; 5] = [
        IRMeshLayoutItem {
            field: IRLayoutField::Position,
            sample_type: IRLayoutSampleType::Float,
            samples: 3, // floats
            stride_bytes: 48,
            offset_bytes: 0,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::Normal,
            sample_type: IRLayoutSampleType::Float,
            samples: 3, // floats
            stride_bytes: 48,
            offset_bytes: 12,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::TexCoord,
            sample_type: IRLayoutSampleType::Float,
            samples: 2, // floats
            stride_bytes: 48,
            offset_bytes: 24,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::BoneIndices,
            sample_type: IRLayoutSampleType::U32,
            samples: 4, // u32
            stride_bytes: 48,
            offset_bytes: 32,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::BoneWeights,
            sample_type: IRLayoutSampleType::Float,
            samples: 4, // floats
            stride_bytes: 48,
            offset_bytes: 48,
        },
    ];

    const SKINNING_TANGENT: [IRMeshLayoutItem; 7] = [
        IRMeshLayoutItem {
            field: IRLayoutField::Position,
            sample_type: IRLayoutSampleType::Float,
            samples: 3, // floats
            stride_bytes: 72,
            offset_bytes: 0,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::Normal,
            sample_type: IRLayoutSampleType::Float,
            samples: 3, // floats
            stride_bytes: 72,
            offset_bytes: 12,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::TexCoord,
            sample_type: IRLayoutSampleType::Float,
            samples: 2, // floats
            stride_bytes: 72,
            offset_bytes: 24,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::Tangent,
            sample_type: IRLayoutSampleType::Float,
            samples: 3, // floats
            stride_bytes: 72,
            offset_bytes: 32,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::Bitangent,
            sample_type: IRLayoutSampleType::Float,
            samples: 3, // floats
            stride_bytes: 72,
            offset_bytes: 44,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::BoneIndices,
            sample_type: IRLayoutSampleType::U32,
            samples: 4, // u32
            stride_bytes: 72,
            offset_bytes: 56,
        },
        IRMeshLayoutItem {
            field: IRLayoutField::BoneWeights,
            sample_type: IRLayoutSampleType::Float,
            samples: 4, // floats
            stride_bytes: 72,
            offset_bytes: 72,
        },
    ];

    match (tangent_valid, skinning) {
        (false, false) => &BASE,
        (true, false) => &TANGENT,
        (false, true) => &SKINNING,
        (true, true) => &SKINNING_TANGENT,
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

pub const MAX_BONES_INFLUENCE: usize = 4;

#[derive(Serialize, Deserialize, Clone)]
pub struct IRSubMesh {
    // Raw bytes of vertices
    #[serde(with = "serde_bytes")]
    pub vertices: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub indices: Vec<u8>,
    pub material: Option<AssetID>,
    pub bounds: IRMeshBounds,
    pub topology: IRTopology,

    // The base layout is:
    //      pos: Vec3<f32>
    //      norm: Vec3<f32>
    //      tex_coord: Vec2<f32>
    //
    // If tangent space is valid, the following layout is added to the base one:
    //      tangent: Vec3<f32>
    //      bitangent: Vec3<f32>
    pub tangent_valid: bool,

    // If skinning is enabled, the following layout is added to the base one:
    //      bone_indices: Vec4<u32>
    //      bone_weights: Vec4<f32>
    pub skinning: bool,
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
            tangent_valid: false,
            skinning: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum IRLayoutField {
    Position,
    Normal,
    Tangent,
    Bitangent,
    TexCoord,
    BoneIndices,
    BoneWeights,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum IRLayoutSampleType {
    Float,
    U32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IRMeshLayoutItem {
    pub field: IRLayoutField,
    pub sample_type: IRLayoutSampleType,
    pub samples: usize,
    pub stride_bytes: usize,
    pub offset_bytes: usize,
}

impl IRMesh {
    pub fn memory_usage(&self) -> usize {
        let mut sum = size_of::<IRMesh>();
        sum += self.submesh.capacity() * size_of::<IRSubMesh>();
        for submesh in &self.submesh {
            sum += submesh.vertices.capacity() * size_of::<u8>();
            sum += submesh.indices.capacity() * size_of::<u8>();
        }
        sum
    }
}
