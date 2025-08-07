use log::debug;
use serde::{Deserialize, Serialize};
use std::ptr::NonNull;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub mod factory;
pub mod manager;
pub mod reader;
pub(crate) mod registry;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
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
pub struct AssetID(String);

impl AssetID {
    pub fn new(str: String) -> AssetID {
        AssetID(str)
    }
}

impl std::fmt::Display for AssetID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AssetID({})", self.0)
    }
}

impl Default for AssetType {
    fn default() -> Self {
        AssetType::Unknown
    }
}

pub trait TypedAsset {
    const TYPE: AssetType;
}

#[derive(Debug, Clone)]
pub struct Asset {
    t: AssetType,
    in_use: Arc<AtomicBool>,
    ptr: NonNull<()>,
}

impl Asset {
    pub fn new(t: AssetType, in_use: Arc<AtomicBool>, ptr: NonNull<()>) -> Asset {
        Asset { t, in_use, ptr }
    }

    pub fn cast<'a, T: TypedAsset>(&self) -> &'a T {
        if self.t != T::TYPE {
            panic!("Asset type mismatch");
        }

        unsafe { &*self.ptr.as_ptr().cast::<T>() }
    }
}

impl Drop for Asset {
    fn drop(&mut self) {
        debug!("Asset of type {:?} is being dropped", self.t);

        // Tell the manager that this asset is no longer in use
        if self.in_use.load(std::sync::atomic::Ordering::SeqCst) {
            panic!("Asset is still in use");
        }
    }
}

unsafe impl Send for Asset {}
unsafe impl Sync for Asset {}
