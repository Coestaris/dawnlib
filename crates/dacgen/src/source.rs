use crate::deep_hash::{deep_hash_bytes, with_std, DeepHash, DeepHashCtx};
use dawn_dac::ChecksumAlgorithm;
use dawn_util::profile::Measure;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
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

pub struct SerializedURL(Url);

impl Serialize for SerializedURL {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.as_str())
    }
}

impl<'de> Deserialize<'de> for SerializedURL {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let url = Url::parse(&s).map_err(serde::de::Error::custom)?;
        Ok(SerializedURL(url))
    }
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
    pub fn read(&self, cache_dir: &Path, cwd: &Path) -> Result<Vec<u8>, String> {
        let path = self.as_path(cache_dir, cwd)?;
        std::fs::read(path).map_err(|e| e.to_string())
    }

    pub fn as_path(&self, cache_dir: &Path, cwd: &Path) -> Result<PathBuf, String> {
        match self {
            SourceRef::File(path) => Ok(if path.is_absolute() {
                path.clone()
            } else {
                cwd.join(path)
            }),
            SourceRef::Url { url, cache } => {
                let segments: PathBuf = url.path().into();
                let base_name = segments
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| format!("Invalid URL path: {}", url))?;
                let hash = deep_hash_bytes(url.as_str().as_bytes(), ChecksumAlgorithm::Blake3)
                    .map_err(|e| format!("Hashing error: {}", e))?
                    .hex_string();
                let dl_dir = cache_dir.join("dl");
                let filename = dl_dir.join(format!("{}_{}", hash, base_name));

                // Download the file if it doesn't exist
                if !filename.exists() {
                    let _measure = Measure::new(format!("Downloaded {} in", url));
                    let response = reqwest::blocking::get(url.as_str())
                        .map_err(|e| format!("Failed to download {}: {}", url, e))?;
                    if !response.status().is_success() {
                        return Err(format!(
                            "Failed to download {}: HTTP {}",
                            url,
                            response.status()
                        ));
                    }

                    let content = response.bytes().map_err(|e| e.to_string())?;
                    std::fs::create_dir_all(dl_dir)
                        .map_err(|e| format!("Failed to create cache dir: {}", e))?;
                    std::fs::write(&filename, &content)
                        .map_err(|e| format!("Failed to write to cache file: {}", e))?;
                }

                Ok(filename)
            }
        }
    }
}

impl DeepHash for SourceRef {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> Result<(), String> {
        match self {
            SourceRef::File(path) => {
                0u8.hash(state);
                path.hash(state);
                let path = self.as_path(ctx.cache_dir.as_path(), ctx.cwd.as_path())?;
                let content = std::fs::read(path).map_err(|e| e.to_string())?;
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
