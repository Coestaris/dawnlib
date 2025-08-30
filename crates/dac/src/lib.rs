use dawn_assets::{AssetHeader, AssetID};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::time::SystemTime;
use thiserror::Error;

pub mod reader;
pub mod writer;

// DAC file format (Dawn Asset Container):
// - 3 bytes: "DAC" magic
// - Repeated segments:
//   - 1 byte: segment type magic
//   - 4 bytes: segment length (u32 little-endian)
//   - N bytes: segment data
//
// Segment types:
// - 0x0: TOC (Table of contents) segment
//   - Serialized TOC structure (HashMap<AssetID, Record>)
// - 0x1: Manifest segment
//   - Serialized Manifest structure
// - 0x2: Data segment
//   - Concatenated raw asset data

pub(crate) const DAC_MAGIC: &[u8; 3] = b"DAC";
pub(crate) const TOC_MAGIC: u8 = 0x0;
pub(crate) const MANIFEST_MAGIC: u8 = 0x1;
pub(crate) const DATA_MAGIC: u8 = 0x2;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum CompressionMode {
    None,
    Brotli,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Record {
    offset: u32,
    length: u32,
    compression: CompressionMode,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TOC(HashMap<AssetID, Record>);

#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Compression error: {0}")]
    CompressionError(anyhow::Error),
    #[error("Serialization error: {0}")]
    SerializationError(anyhow::Error),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Size overflow")]
    SizeOverflow,
    #[error("Invalid magic number")]
    InvalidMagic,
    #[error("Segment not found")]
    SegmentNotFound,
    #[error("Asset not found: {0}")]
    AssetNotFound(AssetID),
    #[error("Deserialization error: {0}")]
    DeserializationError(anyhow::Error),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash)]
pub enum ReadMode {
    Flat,
    Recursive,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Hash)]
pub enum ChecksumAlgorithm {
    Blake3,
    Md5,
    SHA256,
}

impl Display for ChecksumAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blake3 => write!(f, "Blake3"),
            Self::Md5 => write!(f, "MD5"),
            Self::SHA256 => write!(f, "SHA256"),
        }
    }
}

#[derive(Debug, Clone, Hash)]
pub enum CompressionLevel {
    None,
    Fast,
    Default,
    Best,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
    pub extras: Option<String>,
}

impl Version {
    pub fn new(major: u16, minor: u16, patch: u16, extras: Option<String>) -> Self {
        Self {
            major,
            minor,
            patch,
            extras,
        }
    }
}
impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(extras) = &self.extras {
            write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, extras)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    // File information
    pub author: Option<String>,
    pub description: Option<String>,
    pub version: Option<Version>,
    pub license: Option<String>,

    // Technical information
    pub tool: String,
    pub tool_version: Version,
    pub created: SystemTime,
    pub read_mode: ReadMode,
    pub checksum_algorithm: ChecksumAlgorithm,
    pub headers: Vec<AssetHeader>,
}

impl Manifest {
    pub fn tree(&self, id: AssetID, callback: &impl Fn(&AssetID, &AssetHeader, usize)) {
        pub fn tree_inner(
            manifest: &Manifest,
            depth: usize,
            id: &AssetID,
            callback: &impl Fn(&AssetID, &AssetHeader, usize),
        ) {
            let asset = manifest.headers.iter().find(|h| h.id == id.clone());
            if let Some(asset) = asset {
                callback(id, asset, depth);

                for dep in &asset.dependencies {
                    tree_inner(manifest, depth + 1, dep, callback);
                }
            }
        }

        tree_inner(self, 0, &id, callback)
    }
}

#[cfg(any())]
pub mod serialize_backend {
    use serde::{Deserialize, Serialize};
    use toml;

    pub fn serialize<T: Serialize>(object: &T) -> Result<Vec<u8>, String> {
        toml::to_string(object)
            .map(|s| s.into_bytes())
            .map_err(|e| e.to_string())
    }

    pub fn deserialize<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, String> {
        toml::from_slice(bytes).map_err(|e| e.to_string())
    }
}

#[cfg(all())]
pub mod serialize_backend {
    use bincode;
    use serde::de::DeserializeOwned;
    use serde::Serialize;

    pub fn serialize<T: Serialize>(object: &T) -> anyhow::Result<Vec<u8>> {
        let data = bincode::serde::encode_to_vec(object, bincode::config::standard())?;
        Ok(data)
    }

    pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> anyhow::Result<T> {
        let (object, _) = bincode::serde::decode_from_slice(bytes, bincode::config::standard())?;
        Ok(object)
    }
}

pub mod compression_backend {
    use crate::CompressionLevel;
    use brotli::enc::SliceWrapper;
    use brotli::enc::{
        backward_references::UnionHasher,
        encode::BrotliEncoderMaxCompressedSizeMulti,
        multithreading::compress_multi,
        threading::{Owned, SendAlloc},
        writer::CompressorWriter,
        BrotliEncoderParams, StandardAlloc,
    };
    use std::io::{Read, Write};
    use std::sync::Arc;

    pub fn compress(data: &[u8], level: CompressionLevel) -> anyhow::Result<Vec<u8>> {
        // Why bother compressing if the level is None?
        if matches!(level, CompressionLevel::None) {
            return Ok(data.to_vec());
        }

        fn to_params(level: CompressionLevel) -> BrotliEncoderParams {
            let mut p = BrotliEncoderParams::default();
            match level {
                CompressionLevel::Fast => {
                    p.quality = 3;
                    p.lgwin = 20;
                }
                CompressionLevel::Default => {
                    p.quality = 6;
                    p.lgwin = 22;
                }
                CompressionLevel::Best => {
                    p.quality = 11;
                    p.lgwin = 22;
                }
                CompressionLevel::None => {
                    unreachable!()
                }
            }

            p
        }

        struct ArcSlice(Arc<[u8]>);
        impl SliceWrapper<u8> for ArcSlice {
            #[inline]
            fn slice(&self) -> &[u8] {
                &self.0
            }
        }

        const THRESHOLD: usize = 64 * 1024;
        let params = to_params(level);

        // Do not use multi-threading if the data is too small.
        // It's faster to use a single thread.
        if data.len() <= THRESHOLD {
            let mut w = CompressorWriter::with_params(Vec::new(), 64 * 1024, &params);
            w.write_all(data)?;
            return Ok(w.into_inner());
        }

        // Multi-threading is used.
        // Making the owing input to be 'static
        let owned_buf: Arc<[u8]> = Arc::from(data); // одна аллокация и копия входа
        let mut owned_input = Owned::new(ArcSlice(owned_buf)); // Owned<SliceW>

        // Select threads count
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .max(2);
        let mut out =
            vec![
                0u8;
                BrotliEncoderMaxCompressedSizeMulti(owned_input.view().slice().len(), threads)
            ];

        // Prepare per-thread structures
        let mut per_thread: Vec<
            SendAlloc<
                brotli::enc::threading::CompressionThreadResult<StandardAlloc>,
                UnionHasher<StandardAlloc>,
                StandardAlloc,
                _,
            >,
        > = (0..threads)
            .map(|_| SendAlloc::new(StandardAlloc::default(), UnionHasher::default()))
            .collect();

        // Call the multi-threaded compression function
        let written = compress_multi(&params, &mut owned_input, &mut out[..], &mut per_thread[..])
            .map_err(|e| anyhow::anyhow!("Compress multi failed {:?}", e))?;

        out.truncate(written);
        Ok(out)
    }

    pub fn decompress(data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let mut decompressed = Vec::new();
        let mut reader = brotli::Decompressor::new(data, 4096);
        reader.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
}
