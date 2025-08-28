use crate::gl::entities::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use crate::gl::entities::element_array_buffer::{ElementArrayBuffer, ElementArrayBufferUsage};
use crate::gl::entities::vertex_array::VertexArray;
use crate::passes::result::PassExecuteResult;
use dawn_assets::ir::mesh::{IRIndexType, IRMesh, IRSubMesh, IRTopology, IRVertex};
use dawn_assets::{Asset, AssetCastable, AssetID, AssetMemoryUsage};
use glam::Vec3;
use std::collections::HashMap;
use std::ptr::slice_from_raw_parts;

pub struct SubMesh {
    pub material: Option<Asset>,
    pub min: Vec3,
    pub max: Vec3,
    pub index_offset: usize, // In units (u32 or u16)
    pub index_count: usize,  // In units (u32 or u16)
}

pub struct TopologyBucket {
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
    topology: IRTopology,
    index_type: IRIndexType,
    irs: Vec<IRSubMesh>,
}

impl IRBucket {
    pub fn into_bucket(self, deps: &HashMap<AssetID, Asset>) -> Result<TopologyBucket, String> {
        let vao = VertexArray::new(self.topology, self.index_type.clone())
            .map_err(|e| format!("Failed to create VertexArray: {}", e))?;
        let mut vbo =
            ArrayBuffer::new().map_err(|e| format!("Failed to create ArrayBuffer: {}", e))?;
        let mut ebo = ElementArrayBuffer::new()
            .map_err(|e| format!("Failed to create ElementArrayBuffer: {}", e))?;

        let vao_binding = vao.bind();
        let vbo_binding = vbo.bind();
        let ebo_binding = ebo.bind();

        let mut joined_vertices = Vec::new();
        let mut joined_indices = Vec::new();
        let mut index_offset = 0;

        for submesh in &self.irs {
            joined_vertices.extend_from_slice(&submesh.vertices);
            match self.index_type {
                IRIndexType::U16 => unimplemented!(),
                IRIndexType::U32 => {
                    for chunk in submesh.indices.chunks(4) {
                        let array = slice_from_raw_parts(chunk.as_ptr(), 4);
                        let array = unsafe { &*(array as *const [u8; 4]) };
                        let index = u32::from_le_bytes(*array) + index_offset as u32;
                        joined_indices.extend_from_slice(&index.to_le_bytes());
                    }
                }
            }

            index_offset += submesh.vertices.len() / std::mem::size_of::<IRVertex>();
        }

        vbo_binding
            .feed(&joined_vertices, ArrayBufferUsage::StaticDraw)
            .map_err(|e| format!("Failed to feed vertices to ArrayBuffer: {}", e))?;
        ebo_binding
            .feed(&joined_indices, ElementArrayBufferUsage::StaticDraw)
            .map_err(|e| format!("Failed to feed indices to ElementArrayBuffer: {}", e))?;

        for (i, layout) in IRVertex::layout().iter().enumerate() {
            vao_binding
                .setup_attribute(i, layout)
                .map_err(|e| format!("Failed to enable attribute in VertexArray: {}", e))?;
        }

        drop(vbo_binding);
        drop(ebo_binding);
        drop(vao_binding);

        let divider = match self.index_type {
            IRIndexType::U16 => 2,
            IRIndexType::U32 => 4,
        };

        let mut submesh = Vec::with_capacity(self.irs.len());
        let mut submesh_offset = 0;
        for submesh_ir in self.irs {
            let material = match &submesh_ir.material {
                None => None,
                Some(id) => match deps.get(id) {
                    None => return Err(format!("Material with ID '{}' not found for submesh", id)),
                    Some(mat) => Some(mat.clone()),
                },
            };

            submesh.push(SubMesh {
                material,
                min: submesh_ir.bounds.min(),
                max: submesh_ir.bounds.max(),
                index_offset: submesh_offset / divider,
                index_count: submesh_ir.indices.len() / divider,
            });

            submesh_offset += submesh_ir.indices.len();
        }

        Ok(TopologyBucket {
            vao,
            vbo,
            ebo,
            indices_count: submesh_offset / divider,
            submesh,
        })
    }
}

impl AssetCastable for Mesh {}

impl Mesh {
    pub fn from_ir(
        ir: IRMesh,
        deps: HashMap<AssetID, Asset>,
    ) -> Result<(Self, AssetMemoryUsage), String> {
        // Group submeshes by topology
        let mut ir_buckets = HashMap::new();
        for submesh in ir.submesh {
            let bucket = ir_buckets
                .entry(submesh.topology.clone())
                .or_insert(IRBucket {
                    topology: submesh.topology.clone(),
                    index_type: ir.index_type.clone(),
                    irs: Vec::new(),
                });
            bucket.irs.push(submesh);
        }

        let mut buckets = Vec::with_capacity(ir_buckets.len());
        for bucket in ir_buckets.into_values() {
            buckets.push(bucket.into_bucket(&deps)?);
        }

        Ok((
            Mesh {
                buckets,
                min: ir.bounds.min(),
                max: ir.bounds.max(),
            },
            AssetMemoryUsage::new(size_of::<Mesh>(), 0),
        ))
    }
}

impl Mesh {
    #[inline(always)]
    pub fn draw(
        &self,
        on_submesh: impl Fn(&SubMesh) -> (bool, PassExecuteResult),
    ) -> PassExecuteResult {
        let mut result = PassExecuteResult::default();

        for bucket in &self.buckets {
            let binding = bucket.vao.bind();
            for submesh in &bucket.submesh {
                let (skip, new_result) = on_submesh(submesh);
                result += new_result;

                if skip {
                    continue;
                }
                result += binding.draw_elements(submesh.index_count, submesh.index_offset);
            }
        }

        result
    }
}
