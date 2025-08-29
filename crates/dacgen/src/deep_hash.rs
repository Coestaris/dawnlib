use crate::WriterError;
use dawn_assets::AssetChecksum;
use dawn_dac::ChecksumAlgorithm;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

pub struct DeepHashCtx {
    pub cwd: PathBuf,
    pub cache_dir: PathBuf,
}

impl DeepHashCtx {
    pub fn new(cache_dir: PathBuf, cwd: PathBuf) -> Self {
        DeepHashCtx { cache_dir, cwd }
    }
}

struct Blake3Hasher(blake3::Hasher);

impl Hasher for Blake3Hasher {
    fn finish(&self) -> u64 {
        // Not used
        0
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }
}

impl Blake3Hasher {
    fn new(hasher: blake3::Hasher) -> Self {
        Blake3Hasher(hasher)
    }

    fn into_inner(self) -> blake3::Hasher {
        self.0
    }
}

pub struct DeepHasher {
    algorithm: ChecksumAlgorithm,
    hash: AssetChecksum,
}

impl DeepHasher {
    pub fn new(algorithm: ChecksumAlgorithm) -> Self {
        DeepHasher {
            algorithm,
            hash: Default::default(),
        }
    }

    pub fn update_object<T: DeepHash>(
        &mut self,
        obj: &T,
        cache_dir: PathBuf,
        cwd: PathBuf,
    ) -> anyhow::Result<()> {
        let mut ctx = DeepHashCtx::new(cache_dir, cwd);
        match self.algorithm {
            ChecksumAlgorithm::Blake3 => {
                let hasher = blake3::Hasher::new();
                let mut hasher = Blake3Hasher::new(hasher);
                hasher.write(self.hash.as_slice());
                obj.deep_hash(&mut hasher, &mut ctx)?;
                let hasher = hasher.into_inner();
                let hash = hasher.finalize();
                self.hash = AssetChecksum::from_bytes(hash.as_bytes());
                Ok(())
            }
            #[cfg(feature = "hash_md5")]
            ChecksumAlgorithm::Md5 => {
                unimplemented!()
            }
            #[cfg(feature = "hash_sha2")]
            ChecksumAlgorithm::SHA256 => {
                unimplemented!()
            }
            _ => Err(WriterError::UnsupportedChecksumAlgorithm(self.algorithm).into()),
        }
    }

    pub(crate) fn finalize(self) -> AssetChecksum {
        self.hash
    }
}

pub(crate) fn hash_bytes(
    bytes: &[u8],
    algorithm: ChecksumAlgorithm,
) -> Result<AssetChecksum, WriterError> {
    match algorithm {
        ChecksumAlgorithm::Blake3 => {
            let hash = blake3::hash(bytes);
            Ok(AssetChecksum::from_bytes(hash.as_bytes()))
        }
        #[cfg(feature = "hash_md5")]
        ChecksumAlgorithm::Md5 => {
            unimplemented!()
        }
        #[cfg(feature = "hash_sha2")]
        ChecksumAlgorithm::SHA256 => {
            unimplemented!()
        }
        _ => Err(WriterError::UnsupportedChecksumAlgorithm(algorithm)),
    }
}

pub trait DeepHash {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()>;
}

pub fn with_std<T: Hash, H: Hasher>(value: &T, state: &mut H) {
    value.hash(state);
}

macro_rules! impl_basic {
    ($($t:ty),*) => {
        $(
            impl DeepHash for $t {
                #[inline]
                fn deep_hash<H: Hasher>(&self, state: &mut H, _: &mut DeepHashCtx) -> anyhow::Result<()> {
                    self.hash(state);
                    Ok(())
                }
            }
        )*
    };
}

impl_basic!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, String, char, bool);

impl<T: DeepHash> DeepHash for Vec<T> {
    fn deep_hash<H: Hasher>(&self, state: &mut H, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        for item in self {
            item.deep_hash(state, ctx)?;
        }
        Ok(())
    }
}

impl<K: DeepHash + Ord> DeepHash for HashSet<K> {
    fn deep_hash<H: Hasher>(&self, state: &mut H, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        // Sort keys for consistent hashing
        let mut keys: Vec<&K> = self.iter().collect();
        keys.sort();
        for key in keys {
            key.deep_hash(state, ctx)?;
        }
        Ok(())
    }
}

impl<K: DeepHash + Ord + Hash, V: DeepHash> DeepHash for HashMap<K, V> {
    fn deep_hash<H: Hasher>(&self, state: &mut H, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        // Sort keys for consistent hashing
        let mut keys: Vec<&K> = self.keys().collect();
        keys.sort();
        keys.len().hash(state);
        for key in keys {
            key.deep_hash(state, ctx)?;
            if let Some(value) = self.get(key) {
                value.deep_hash(state, ctx)?;
            }
        }
        Ok(())
    }
}

impl DeepHash for f32 {
    fn deep_hash<T: Hasher>(&self, state: &mut T, _: &mut DeepHashCtx) -> anyhow::Result<()> {
        // Hash the bit representation of the float
        self.to_bits().hash(state);
        Ok(())
    }
}

impl DeepHash for f64 {
    fn deep_hash<T: Hasher>(&self, state: &mut T, _: &mut DeepHashCtx) -> anyhow::Result<()> {
        // Hash the bit representation of the float
        self.to_bits().hash(state);
        Ok(())
    }
}

impl<'a, T: DeepHash> DeepHash for &'a T {
    fn deep_hash<H: Hasher>(&self, state: &mut H, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        (*self).deep_hash(state, ctx)
    }
}

impl<const N: usize> DeepHash for [f32; N] {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        for v in self {
            (*v).deep_hash(state, ctx)?;
        }
        Ok(())
    }
}

impl<T: DeepHash> DeepHash for Option<T> {
    fn deep_hash<H: Hasher>(&self, state: &mut H, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        match self {
            Some(value) => {
                state.write_u8(1);
                value.deep_hash(state, ctx)?;
            }
            None => {
                state.write_u8(0);
            }
        };
        Ok(())
    }
}
