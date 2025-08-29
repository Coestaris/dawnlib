use crate::deep_hash::{DeepHash, DeepHashCtx};
use dawn_dac::{ChecksumAlgorithm, CompressionLevel, ReadMode};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct WriteConfig {
    pub read_mode: ReadMode,
    pub checksum_algorithm: ChecksumAlgorithm,
    pub compression_level: CompressionLevel,
    pub cache_dir: PathBuf,
    pub author: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
}

impl DeepHash for ChecksumAlgorithm {
    fn deep_hash<T: Hasher>(&self, state: &mut T, _ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.hash(state);
        Ok(())
    }
}

impl DeepHash for ReadMode {
    fn deep_hash<T: Hasher>(&self, state: &mut T, _ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.hash(state);
        Ok(())
    }
}

impl DeepHash for CompressionLevel {
    fn deep_hash<T: Hasher>(&self, state: &mut T, _ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.hash(state);
        Ok(())
    }
}

impl DeepHash for WriteConfig {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        self.read_mode.deep_hash(state, ctx)?;
        self.checksum_algorithm.deep_hash(state, ctx)?;
        self.compression_level.deep_hash(state, ctx)?;
        // Do not hash the cache dir path contents, only the path itself
        self.cache_dir.hash(state);
        self.author.deep_hash(state, ctx)?;
        self.description.deep_hash(state, ctx)?;
        self.version.deep_hash(state, ctx)?;
        self.license.deep_hash(state, ctx)?;
        Ok(())
    }
}
