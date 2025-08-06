use crate::resources::r#ref::ResourceRef;
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Unknown,

    // Shader types
    ShaderGLSL,
    ShaderSPIRV,
    ShaderHLSL,

    // Audio types
    AudioMIDI,
    AudioFLAC,
    AudioWAV,
    AudioOGG,

    // Image types
    ImagePNG,
    ImageJPEG,
    ImageBMP,

    // Font types
    FontTTF,
    FontOTF,

    // Model types
    ModelOBJ,
    ModelGLTF,
    ModelFBX,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceID(String);

impl std::fmt::Display for ResourceID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResourceID({})", self.0)
    }
}

impl Default for ResourceType {
    fn default() -> Self {
        ResourceType::Unknown
    }
}

pub struct Resource {
    type_id: TypeId,
    cell: ResourceRef,
}

impl Resource {
    pub fn new<T>(cell: ResourceRef) -> Self
    where
        T: Any + Send + Sync,
    {
        Resource {
            type_id: TypeId::of::<T>(),
            cell,
        }
    }

    fn deref<'a, T>(self) -> &'a T
    where
        T: Any + Send + Sync,
    {
        if self.type_id != TypeId::of::<T>() {
            panic!("Resource type mismatch");
        }

        unsafe { self.cell.deref::<T>() }
    }
}
