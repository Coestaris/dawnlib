use crate::AssetID;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::mem::offset_of;

pub const IR_MAX_BONE_INFLUENCES: usize = 4;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[repr(C)]
#[repr(packed)]
pub struct IRVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
    pub tex_coord: [f32; 2],
    pub bone_indices: [u32; IR_MAX_BONE_INFLUENCES],
    pub bone_weights: [f32; IR_MAX_BONE_INFLUENCES],
}

impl IRVertex {
    fn position(&self) -> Vec3 {
        Vec3::from(self.position)
    }

    fn normal(&self) -> Vec3 {
        Vec3::from(self.normal)
    }

    fn tangent(&self) -> Vec3 {
        Vec3::from(self.tangent)
    }

    fn bitangent(&self) -> Vec3 {
        Vec3::from(self.bitangent)
    }

    fn tex_coord(&self) -> Vec2 {
        Vec2::from(self.tex_coord)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IRMesh {
    // All the geometry hierarchy is baked into the single World-Space mesh.
    pub vertices: Vec<IRVertex>,
    pub indices: Vec<u32>,
    pub material: AssetID,
}

impl Default for IRMesh {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            material: AssetID::default(),
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

impl IRVertex {
    fn layout() -> [IRMeshLayout; 7] {
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
                field: IRMeshField::Tangent,
                sample_type: IRMeshLayoutSampleType::Float,
                samples: 3, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, tangent),
            },
            IRMeshLayout {
                field: IRMeshField::Bitangent,
                sample_type: IRMeshLayoutSampleType::Float,
                samples: 3, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, bitangent),
            },
            IRMeshLayout {
                field: IRMeshField::TexCoord,
                sample_type: IRMeshLayoutSampleType::Float,
                samples: 2, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, tex_coord),
            },
            IRMeshLayout {
                field: IRMeshField::BoneIndices,
                sample_type: IRMeshLayoutSampleType::U32,
                samples: IR_MAX_BONE_INFLUENCES,
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, bone_indices),
            },
            IRMeshLayout {
                field: IRMeshField::BoneWeights,
                sample_type: IRMeshLayoutSampleType::Float,
                samples: IR_MAX_BONE_INFLUENCES, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, bone_weights),
            },
        ]
    }
}
