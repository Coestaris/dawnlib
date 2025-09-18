use crate::gl::raii::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use crate::gl::raii::element_array_buffer::{ElementArrayBuffer, ElementArrayBufferUsage};
use crate::gl::raii::vertex_array::VertexArray;
use crate::passes::result::RenderResult;
use dawn_assets::ir::mesh::{layout_of_submesh, IRIndexType, IRMesh, IRSubMesh, IRTopology};
use dawn_assets::{Asset, AssetCastable, AssetID, AssetMemoryUsage};
use glam::Vec3;
use log::debug;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MeshError {
    #[error("Material with ID '{0}' not found for submesh")]
    MaterialNotFound(AssetID),
    #[error("Failed to allocate VertexArray")]
    VertexArrayAllocationFailed,
    #[error("Failed to allocate ArrayBuffer")]
    ArrayBufferAllocationFailed,
    #[error("Failed to allocate ElementArrayBuffer")]
    ElementArrayBufferAllocationFailed,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BucketKey {
    pub topology: IRTopology,
    pub skinning: bool,
    pub tangent_valid: bool,
}

pub struct SubMesh {
    pub material: Option<Asset>,
    pub min: Vec3,
    pub max: Vec3,
    pub index_offset: usize,  // In units (u32 or u16)
    pub vertex_offset: usize, // In units (Vertex size)
    pub index_count: usize,   // In units (u32 or u16)
}

pub struct TopologyBucket {
    pub key: BucketKey,
    pub vao: VertexArray,
    pub vbo: ArrayBuffer,
    pub ebo: ElementArrayBuffer,
    pub indices_count: usize, // In units (u32 or u16)
    pub submesh: Vec<SubMesh>,
}

pub struct Mesh {
    pub buckets: Vec<TopologyBucket>,
    pub min: Vec3,
    pub max: Vec3,
}

struct IRBucket {
    key: BucketKey,
    index_type: IRIndexType,
    irs: Vec<IRSubMesh>,
}

impl IRBucket {
    pub fn into_bucket(
        self,
        gl: Arc<glow::Context>,
        deps: &HashMap<AssetID, Asset>,
    ) -> Result<TopologyBucket, MeshError> {
        let vao = VertexArray::new(
            gl.clone(),
            self.key.topology.clone(),
            self.index_type.clone(),
        )
        .ok_or(MeshError::VertexArrayAllocationFailed)?;
        let mut vbo = ArrayBuffer::new(gl.clone()).ok_or(MeshError::ArrayBufferAllocationFailed)?;
        let mut ebo = ElementArrayBuffer::new(gl.clone())
            .ok_or(MeshError::ElementArrayBufferAllocationFailed)?;

        let vao_binding = vao.bind();
        let vbo_binding = vbo.bind();
        let ebo_binding = ebo.bind();

        let joined_vertices = self
            .irs
            .iter()
            .flat_map(|submesh| submesh.raw_vertices().to_vec())
            .collect::<Vec<u8>>();
        let joined_indices = self
            .irs
            .iter()
            .flat_map(|submesh| submesh.raw_indices().to_vec())
            .collect::<Vec<u8>>();

        vbo_binding.feed(&joined_vertices, ArrayBufferUsage::StaticDraw);
        ebo_binding.feed(&joined_indices, ElementArrayBufferUsage::StaticDraw);

        let layout = layout_of_submesh(self.key.tangent_valid, self.key.skinning);
        for (i, layout) in layout.iter().enumerate() {
            vao_binding.setup_attribute(i as u32, layout);
        }

        drop(vbo_binding);
        drop(ebo_binding);
        drop(vao_binding);

        let divider = match self.index_type {
            IRIndexType::U16 => 2,
            IRIndexType::U32 => 4,
        };

        let mut submesh = Vec::with_capacity(self.irs.len());
        let mut index_offset = 0;
        let mut vertex_offset = 0;
        for submesh_ir in self.irs {
            let material = match &submesh_ir.material {
                None => None,
                Some(id) => match deps.get(id) {
                    None => return Err(MeshError::MaterialNotFound(id.clone())),
                    Some(mat) => Some(mat.clone()),
                },
            };

            submesh.push(SubMesh {
                material,
                min: submesh_ir.bounds.min(),
                max: submesh_ir.bounds.max(),
                index_offset: index_offset / divider,
                vertex_offset: vertex_offset / layout[0].stride_bytes,
                index_count: submesh_ir.indices.len() / divider,
            });

            index_offset += submesh_ir.indices.len();
            vertex_offset += submesh_ir.vertices.len();
        }

        Ok(TopologyBucket {
            key: self.key.clone(),
            vao,
            vbo,
            ebo,
            indices_count: index_offset / divider,
            submesh,
        })
    }
}

impl AssetCastable for Mesh {}

impl Mesh {
    pub fn from_ir(
        gl: Arc<glow::Context>,
        ir: IRMesh,
        deps: HashMap<AssetID, Asset>,
    ) -> Result<(Self, AssetMemoryUsage), MeshError> {
        debug!("Creating Mesh from IR: {:?}", ir);

        // Group submeshes by topology
        let sum = ir
            .submesh
            .iter()
            .fold(0, |acc, sm| acc + sm.vertices.len() + sm.indices.len());

        // Bucket submeshes by topology, tangent space and skinning
        let mut ir_buckets = HashMap::new();
        for submesh in ir.submesh {
            let key = BucketKey {
                topology: submesh.topology.clone(),
                skinning: submesh.skinning,
                tangent_valid: submesh.tangent_valid,
            };
            let bucket = ir_buckets.entry(key.clone()).or_insert(IRBucket {
                key: key.clone(),
                index_type: ir.index_type.clone(),
                irs: Vec::new(),
            });
            bucket.irs.push(submesh);
        }

        let mut buckets = Vec::with_capacity(ir_buckets.len());
        for bucket in ir_buckets.into_values() {
            buckets.push(bucket.into_bucket(gl.clone(), &deps)?);
        }

        Ok((
            Mesh {
                buckets,
                min: ir.bounds.min(),
                max: ir.bounds.max(),
            },
            AssetMemoryUsage::new(size_of::<Mesh>(), sum),
        ))
    }

    #[inline(always)]
    pub fn draw(
        &self,
        on_bucket: impl Fn(&TopologyBucket) -> (bool, RenderResult),
        on_submesh: impl Fn(&SubMesh) -> (bool, RenderResult),
    ) -> RenderResult {
        let mut result = RenderResult::default();

        for bucket in &self.buckets {
            let (skip, new_result) = on_bucket(bucket);
            result += new_result;
            if skip {
                continue;
            }

            let binding = bucket.vao.bind();
            for submesh in &bucket.submesh {
                let (skip, new_result) = on_submesh(submesh);
                result += new_result;

                if skip {
                    continue;
                }

                result += binding.draw_elements_base_vertex(
                    submesh.index_count,
                    submesh.index_offset,
                    submesh.vertex_offset,
                );
            }
        }

        result
    }
}
