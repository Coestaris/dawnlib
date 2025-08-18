use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IRShaderSourceType {
    Fragment,
    Geometry,
    Vertex,
    Compute,
    TessellationControl,

    /* Precompiled */
    PrecompiledFragment,
    PrecompiledGeometry,
    PrecompiledVertex,
    PrecompiledCompute,
    PrecompiledTessellationControl,
}

impl Default for IRShaderSourceType {
    fn default() -> Self {
        IRShaderSourceType::Fragment
    }
}

/// Internal representation of shader data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IRShader {
    pub compile_options: Vec<String>,
    pub sources: HashMap<IRShaderSourceType, Vec<u8>>,
}

impl Default for IRShader {
    fn default() -> Self {
        IRShader {
            compile_options: vec![],
            sources: Default::default(),
        }
    }
}
