use log::debug;
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;

pub mod factory;
pub mod hub;
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

#[derive(Debug)]
pub struct Asset {
    tid: TypeId,
    rc: Arc<AtomicUsize>,
    ptr: NonNull<()>,
}

impl Clone for Asset {
    fn clone(&self) -> Self {
        // Increment the reference count
        self.rc.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Asset {
            tid: self.tid,
            rc: Arc::clone(&self.rc),
            ptr: self.ptr,
        }
    }
}

impl Asset {
    pub fn new(tid: TypeId, rc: Arc<AtomicUsize>, ptr: NonNull<()>) -> Asset {
        // Increment the reference count
        rc.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Asset { tid, rc, ptr }
    }

    pub fn cast<'a, T: 'static>(&self) -> &'a T {
        if self.tid != TypeId::of::<T>() {
            panic!(
                "Asset type mismatch: expected {:?}, found {:?}",
                TypeId::of::<T>(),
                self.tid
            );
        }

        unsafe { &*self.ptr.as_ptr().cast::<T>() }
    }
}

impl Drop for Asset {
    fn drop(&mut self) {
        // Decrement the reference count
        let rc = self.rc.fetch_sub(1, std::sync::atomic::Ordering::Release);

        debug!("Asset of {:?} is dropped. rc: {}", self.tid, rc);
    }
}

unsafe impl Send for Asset {}
unsafe impl Sync for Asset {}
