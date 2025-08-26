use crate::deep_hash::deep_hash;
use crate::{InstantGuard, UserAssetFile, WriterError};
use dawn_assets::AssetChecksum;
use dawn_dac::container::writer::BinaryAsset;
use dawn_dac::serialize_backend::deserialize;
use dawn_dac::ChecksumAlgorithm;
use log::debug;
use std::path::PathBuf;

pub struct Cache {
    cache_dir: PathBuf,
    cwd: PathBuf,
    checksum_algorithm: ChecksumAlgorithm,
}

impl Cache {
    pub(crate) fn new(
        cache_dir: PathBuf,
        cwd: PathBuf,
        checksum_algorithm: ChecksumAlgorithm,
    ) -> Self {
        Cache {
            cache_dir,
            cwd,
            checksum_algorithm,
        }
    }

    fn get_fn(&self, asset: &UserAssetFile) -> Result<PathBuf, WriterError> {
        let _guard = InstantGuard::new(format!(
            "Calculated deep hash of {} in",
            asset.path.display()
        ));

        let hash = deep_hash(
            asset,
            self.checksum_algorithm,
            self.cache_dir.clone(),
            self.cwd.clone(),
        )?;
        Ok(self.cache_dir.join(hash.hex_string()))
    }

    pub fn get(&self, asset: &UserAssetFile) -> Option<Vec<BinaryAsset>> {
        let _guard = InstantGuard::new(format!("Cache get {:?} computed in", asset.path));

        let cache_path = self.get_fn(asset).ok()?;
        if cache_path.exists() {
            debug!("Cache hit for asset {:?} at {:?}", asset.path, cache_path);
            // Read the cached binaries
            let data = {
                let _guard = InstantGuard::new(format!("Cached Read {:?} computed in", asset.path));
                std::fs::read(&cache_path).ok()?
            };
            // Deserialize the binaries
            let binaries: Vec<BinaryAsset> =  {
                let _guard = InstantGuard::new(format!("Deserialize {:?} computed in", asset.path));
                deserialize(&data).ok()?
            };
            Some(binaries)
        } else {
            debug!("Cache miss for asset {:?} at {:?}", asset.path, cache_path);
            None
        }
    }

    pub fn insert(
        &self,
        asset: &UserAssetFile,
        binaries: &Vec<BinaryAsset>,
    ) -> Result<(), WriterError> {
        let _guard = InstantGuard::new(format!("Cache insert {:?} computed in", asset.path));

        let cache_path = self.get_fn(asset)?;

        // Ensure the cache directory exists
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Serialize the binaries
        let data = dawn_dac::serialize_backend::serialize(binaries)
            .map_err(|e| WriterError::SerializationError(e.to_string()))?;

        // Write to the cache file
        std::fs::write(&cache_path, data)?;
        Ok(())
    }
}
