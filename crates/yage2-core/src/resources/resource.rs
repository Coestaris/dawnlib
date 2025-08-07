use log::debug;
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::ptr::NonNull;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

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

impl ResourceID {
    pub fn new(str: String) -> ResourceID {
        ResourceID(str)
    }
}

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

#[derive(Debug, Clone)]
pub struct Resource {
    type_id: TypeId,
    in_use: Arc<AtomicBool>,
    ptr: NonNull<()>,
}

impl Resource {
    pub fn new<T>(in_use: Arc<AtomicBool>, cell: NonNull<T>) -> Resource
    where
        T: Any + Send + Sync,
    {
        Resource {
            type_id: TypeId::of::<T>(),
            in_use,
            ptr: cell.cast(),
        }
    }

    pub fn cast<'a, T>(&self) -> &'a T
    where
        T: Any + Send + Sync,
    {
        if self.type_id != TypeId::of::<T>() {
            panic!("Resource type mismatch");
        }

        unsafe { &*self.ptr.as_ptr().cast::<T>() }
    }
}

impl Drop for Resource {
    fn drop(&mut self) {
        debug!("Resource of type {:?} is being dropped", self.type_id);

        // Tell the manager that this resource is no longer in use
        if self.in_use.load(std::sync::atomic::Ordering::SeqCst) {
            panic!("Resource is still in use");
        }
    }
}

unsafe impl Send for Resource {}
unsafe impl Sync for Resource {}
