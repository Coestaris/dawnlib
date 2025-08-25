use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::Arc;

pub mod ir;

#[cfg(feature = "hub")]
pub mod binding;
#[cfg(feature = "hub")]
pub mod factory;
#[cfg(feature = "hub")]
pub mod hub;
#[cfg(feature = "hub")]
pub mod reader;
#[cfg(feature = "hub")]
pub(crate) mod registry;
#[cfg(feature = "hub")]
pub mod requests;

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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AssetHeader {
    pub id: AssetID,
    pub asset_type: AssetType,
    pub checksum: AssetChecksum,
    pub dependencies: HashSet<AssetID>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub license: Option<String>,
}

impl Default for AssetHeader {
    fn default() -> Self {
        AssetHeader {
            id: AssetID::default(),
            tags: Vec::new(),
            asset_type: AssetType::Unknown,
            checksum: AssetChecksum::default(),
            dependencies: HashSet::new(),
            license: None,
            author: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    Unknown,
    Shader,
    Texture,
    Audio,
    Notes,
    Material,
    Mesh,
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetType::Unknown => write!(f, "Unknown"),
            AssetType::Shader => write!(f, "Shader"),
            AssetType::Texture => write!(f, "Texture"),
            AssetType::Audio => write!(f, "Audio"),
            AssetType::Notes => write!(f, "Notes"),
            AssetType::Material => write!(f, "Material"),
            AssetType::Mesh => write!(f, "Mesh"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetID(String);

impl AssetID {
    pub fn new(str: String) -> AssetID {
        AssetID(str)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn memory_usage(&self) -> usize {
        self.0.len()
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

#[derive(Debug, Clone)]
struct AssetInner {
    tid: TypeId,
    ptr: NonNull<()>,
}

#[derive(Debug, Clone)]
pub struct Asset(Arc<AssetInner>);

unsafe impl Send for Asset {}
unsafe impl Sync for Asset {}

impl Asset {
    pub fn new(tid: TypeId, ptr: NonNull<()>) -> Asset {
        Asset(Arc::new(AssetInner { tid, ptr }))
    }

    #[allow(dead_code)]
    pub(crate) fn ref_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    pub fn cast<'a, T: AssetCastable>(&self) -> &'a T {
        #[cfg(debug_assertions)]
        if self.0.tid != TypeId::of::<T>() {
            panic!(
                "Asset type mismatch: expected {:?}, found {:?}",
                TypeId::of::<T>(),
                self.0.tid
            );
        }

        unsafe { &*self.0.ptr.as_ptr().cast::<T>() }
    }
}

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
        if asset.0.tid != TypeId::of::<T>() {
            panic!(
                "TypedAsset type mismatch: expected {:?}, found {:?}",
                TypeId::of::<T>(),
                asset.0.tid
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

#[derive(Debug, Clone)]
pub struct AssetMemoryUsage {
    pub ram: usize,
    pub vram: usize,
}

impl AssetMemoryUsage {
    pub fn new(ram: usize, vram: usize) -> Self {
        AssetMemoryUsage { ram, vram }
    }
}
