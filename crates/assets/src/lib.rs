use log::debug;
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub mod factory;
pub mod hub;
pub mod reader;
pub(crate) mod registry;
pub mod ir;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetChecksum([u8; 16]);

impl AssetChecksum {
    pub fn from_bytes(bytes: &[u8]) -> AssetChecksum {
        let mut checksum = [0; 16];
        let len = bytes.len().min(16);
        checksum[..len].copy_from_slice(&bytes[..len]);
        AssetChecksum(checksum)
    }
}

impl Default for AssetChecksum {
    fn default() -> Self {
        AssetChecksum([0; 16])
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetHeader {
    pub id: AssetID,
    #[serde(default)]
    pub tags: Vec<String>,
    pub asset_type: AssetType,
    #[serde(default)]
    pub checksum: AssetChecksum,
    #[serde(default)]
    pub dependencies: Vec<AssetID>,
}

impl Default for AssetHeader {
    fn default() -> Self {
        AssetHeader {
            id: AssetID::default(),
            tags: Vec::new(),
            asset_type: AssetType::Unknown,
            checksum: AssetChecksum::default(),
            dependencies: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    Unknown,
    Shader,
    Texture,
    Audio,
    MIDI,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetID(String);

impl AssetID {
    pub fn new(str: String) -> AssetID {
        AssetID(str)
    }
}

impl From<String> for AssetID {
    fn from(str: String) -> Self {
        AssetID(str)
    }
}

impl From<&str> for AssetID {
    fn from(str: &str) -> Self {
        AssetID(str.to_string())
    }
}

impl Default for AssetID {
    fn default() -> Self {
        AssetID(String::new())
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

pub trait AssetCastable: 'static {}

#[derive(Debug)]
pub struct Asset {
    tid: TypeId,
    rc: Arc<AtomicUsize>,
    ptr: NonNull<()>,
}

impl Clone for Asset {
    fn clone(&self) -> Self {
        // Increment the reference count
        self.rc.fetch_add(1, Ordering::SeqCst);
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
        rc.fetch_add(1, Ordering::SeqCst);

        Asset { tid, rc, ptr }
    }

    pub fn cast<'a, T: AssetCastable>(&self) -> &'a T {
        #[cfg(debug_assertions)]
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
        let rc = self.rc.fetch_sub(1, Ordering::Release);
        debug!("Asset of {:?} is dropped. rc: {}", self.tid, rc);
    }
}

unsafe impl Send for Asset {}
unsafe impl Sync for Asset {}

#[derive(Debug)]
pub struct TypedAsset<T: AssetCastable> {
    inner: Asset,
    _marker: PhantomData<T>,
}

impl<T: AssetCastable> Clone for TypedAsset<T> {
    fn clone(&self) -> Self {
        // Clone the inner Asset, which increments the reference count
        TypedAsset {
            inner: self.inner.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T: AssetCastable> TypedAsset<T> {
    pub fn new(asset: Asset) -> TypedAsset<T> {
        #[cfg(debug_assertions)]
        if asset.tid != TypeId::of::<T>() {
            panic!(
                "TypedAsset type mismatch: expected {:?}, found {:?}",
                TypeId::of::<T>(),
                asset.tid
            );
        }

        TypedAsset {
            inner: asset,
            _marker: PhantomData,
        }
    }

    pub fn cast(&self) -> &T {
        self.inner.cast()
    }
}
