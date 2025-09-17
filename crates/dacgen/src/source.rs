use crate::deep_hash::{hash_bytes, with_std, DeepHash, DeepHashCtx};
use crate::WriterError;
use dawn_dac::ChecksumAlgorithm;
use dawn_util::profile::Measure;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use thiserror::Error;
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CachePolicy {
    UseCache,
    Bypass,
}

impl Default for CachePolicy {
    fn default() -> Self {
        CachePolicy::UseCache
    }
}

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("HTTP error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("HTTP request failed with status code {0}")]
    RequestFailed(u16),
    #[error("URL parse error")]
    URLParseError,
    #[error("Hashing error: {0}")]
    HashingError(#[from] WriterError),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SourceRef {
    File(PathBuf),
    Url {
        url: Url,
        #[serde(default)]
        cache: CachePolicy,
    },
}

impl SourceRef {
    pub fn read(&self, cache_dir: &Path, cwd: &Path) -> Result<Vec<u8>, SourceError> {
        let path = self.as_path(cache_dir, cwd)?;
        Ok(std::fs::read(path)?)
    }

    pub fn as_path(&self, cache_dir: &Path, cwd: &Path) -> Result<PathBuf, SourceError> {
        match self {
            SourceRef::File(path) => Ok(if path.is_absolute() {
                path.clone()
            } else {
                std::path::absolute(cwd.join(path))?
            }),
            SourceRef::Url { url, .. } => {
                let segments: PathBuf = url.path().into();
                let base_name = segments
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| SourceError::URLParseError)?;

                let hash =
                    hash_bytes(url.as_str().as_bytes(), ChecksumAlgorithm::Blake3)?.hex_string();
                let dl_dir = cache_dir.join("dl");
                let filename = dl_dir.join(format!("{}_{}", hash, base_name));

                // Download the file if it doesn't exist
                if !filename.exists() {
                    let _measure = Measure::new(format!("Downloaded {} in", url));
                    let response = reqwest::blocking::get(url.as_str())?;
                    if !response.status().is_success() {
                        return Err(SourceError::RequestFailed(response.status().as_u16()));
                    }

                    let content = response.bytes()?;
                    std::fs::create_dir_all(dl_dir)?;
                    std::fs::write(&filename, &content)?;
                }

                Ok(filename)
            }
        }
    }
}

impl DeepHash for SourceRef {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
        match self {
            SourceRef::File(path) => {
                0u8.hash(state);
                path.hash(state);
                let path = self.as_path(ctx.cache_dir.as_path(), ctx.cwd.as_path())?;
                let content = std::fs::read(path)?;
                content.hash(state);
            }
            SourceRef::Url { url, cache } => {
                1u8.hash(state);
                url.as_str().hash(state);
                with_std(cache, state);
            }
        }

        Ok(())
    }
}
