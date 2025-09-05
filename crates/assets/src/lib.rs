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

/// Deterministic checksum of an asset's data and header.
/// Can be used to verify that an asset hasn't been tampered with.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetChecksum([u8; 16]);

impl AssetChecksum {
    /// Creates a new checksum from the first 16 bytes of the given slice.
    /// If the slice is shorter than 16 bytes, the checksum will be padded with zeros.
    pub fn from_bytes(bytes: &[u8]) -> AssetChecksum {
        let mut checksum = [0; 16];
        let len = bytes.len().min(16);
        checksum[..len].copy_from_slice(&bytes[..len]);
        AssetChecksum(checksum)
    }

    /// Returns the checksum as a slice.
    /// The slice will be 16 bytes long.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Returns the checksum as a hex string.
    /// The string will be 32 characters long.
    pub fn hex_string(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

impl Default for AssetChecksum {
    fn default() -> Self {
        AssetChecksum([0; 16])
    }
}

/// Metadata about an asset
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AssetHeader {
    /// Unique identifier for the asset.
    pub id: AssetID,
    /// Type of the asset.
    pub asset_type: AssetType,
    /// Checksum of the asset's data and header.
    pub checksum: AssetChecksum,
    /// Dependencies of the asset required during loading.
    pub dependencies: HashSet<AssetID>,
    /// Additional tags for the asset.
    pub tags: Vec<String>,
    /// Author of the asset.
    pub author: Option<String>,
    /// Type of the Asset's license or link to it.
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
    Font,
    Dictionary,
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
            AssetType::Font => write!(f, "Font"),
            AssetType::Dictionary => write!(f, "Dictionary"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    _tid: TypeId,
    ptr: NonNull<()>,
}

#[derive(Debug, Clone)]
pub struct Asset(Arc<AssetInner>);

unsafe impl Send for Asset {}
unsafe impl Sync for Asset {}

impl PartialEq for Asset {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Asset {}

impl Asset {
    pub fn new(tid: TypeId, ptr: NonNull<()>) -> Asset {
        Asset(Arc::new(AssetInner { _tid: tid, ptr }))
    }

    #[allow(dead_code)]
    pub(crate) fn ref_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    pub fn cast<'a, T: AssetCastable>(&self) -> &'a T {
        #[cfg(debug_assertions)]
        if self.0._tid != TypeId::of::<T>() {
            panic!(
                "Asset type mismatch: expected {:?}, found {:?}",
                TypeId::of::<T>(),
                self.0._tid
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

impl<T: AssetCastable> PartialEq for TypedAsset<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: AssetCastable> Eq for TypedAsset<T> {}

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
        if asset.0._tid != TypeId::of::<T>() {
            panic!(
                "TypedAsset type mismatch: expected {:?}, found {:?}",
                TypeId::of::<T>(),
                asset.0._tid
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
