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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IRMesh {
    pub vertices: Vec<IRVertex>,
    pub indices: Vec<u32>,
    pub material: String,
}

pub enum IRMeshLayoutSampleType {
    Float,
    U32,
}

pub struct IRMeshLayout {
    pub sample_type: IRMeshLayoutSampleType,
    pub samples: usize,
    pub stride_bytes: usize,
    pub offset_bytes: usize,
}

impl IRVertex {
    fn layout() -> [IRMeshLayout; 7] {
        [
            IRMeshLayout {
                sample_type: IRMeshLayoutSampleType::Float,
                samples: 3, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, position),
            },
            IRMeshLayout {
                sample_type: IRMeshLayoutSampleType::Float,
                samples: 3, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, normal),
            },
            IRMeshLayout {
                sample_type: IRMeshLayoutSampleType::Float,
                samples: 3, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, tangent),
            },
            IRMeshLayout {
                sample_type: IRMeshLayoutSampleType::Float,
                samples: 3, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, bitangent),
            },
            IRMeshLayout {
                sample_type: IRMeshLayoutSampleType::Float,
                samples: 2, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, tex_coord),
            },
            IRMeshLayout {
                sample_type: IRMeshLayoutSampleType::U32,
                samples: IR_MAX_BONE_INFLUENCES,
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, bone_indices),
            },
            IRMeshLayout {
                sample_type: IRMeshLayoutSampleType::Float,
                samples: IR_MAX_BONE_INFLUENCES, // floats
                stride_bytes: size_of::<IRVertex>(),
                offset_bytes: offset_of!(IRVertex, bone_weights),
            },
        ]
    }
}
