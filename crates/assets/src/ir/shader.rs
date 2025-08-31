use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IRShaderSourceKind {
    Fragment,
    Geometry,
    Vertex,
    Compute,
    TessellationControl,
    /* Precompiled */
    // PrecompiledFragment,
    // PrecompiledGeometry,
    // PrecompiledVertex,
    // PrecompiledCompute,
    // PrecompiledTessellationControl,
}

impl Default for IRShaderSourceKind {
    fn default() -> Self {
        IRShaderSourceKind::Fragment
    }
}

/// Internal representation of shader data
#[derive(Serialize, Deserialize, Clone)]
pub struct IRShader {
    pub compile_options: Vec<String>,
    pub sources: HashMap<IRShaderSourceKind, Vec<u8>>,
}

impl Debug for IRShader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IRShader")
            .field("compile_options", &self.compile_options)
            .field("sources_count", &self.sources.len())
            .finish()
    }
}

impl Default for IRShader {
    fn default() -> Self {
        IRShader {
            compile_options: vec![],
            sources: Default::default(),
        }
    }
}

impl IRShader {
    pub fn memory_usage(&self) -> usize {
        let mut sum = 0;
        sum += size_of::<IRShader>();
        for (_, source) in &self.sources {
            sum += source.capacity();
        }
        for option in &self.compile_options {
            sum += option.capacity();
        }
        sum
    }
}
