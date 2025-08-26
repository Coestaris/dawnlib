use dawn_assets::AssetHeader;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::time::SystemTime;

pub mod container;
pub mod reader;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ReadMode {
    Flat,
    Recursive,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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

#[derive(Debug, Clone)]
pub enum CompressionLevel {
    None,
    Fast,
    Default,
    Best,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    // File information
    pub author: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,

    // Technical information
    pub tool: String,
    pub tool_version: String,
    pub created: SystemTime,
    pub read_mode: ReadMode,
    pub checksum_algorithm: ChecksumAlgorithm,
    pub headers: Vec<AssetHeader>,
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
    use bitcode;
    use serde::de::DeserializeOwned;
    use serde::Serialize;

    pub fn serialize<T: Serialize>(object: &T) -> Result<Vec<u8>, String> {
        bitcode::serialize(object).map_err(|e| e.to_string())
    }

    pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
        bitcode::deserialize(bytes).map_err(|e| e.to_string())
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

    pub fn compress(data: &[u8], level: CompressionLevel) -> Result<Vec<u8>, String> {
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
            w.write_all(data).map_err(|e| e.to_string())?;
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
            .map_err(|e| format!("brotli compress_multi failed: {:?}", e))?;

        out.truncate(written);
        Ok(out)
    }

    pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
        let mut decompressed = Vec::new();
        let mut reader = brotli::Decompressor::new(data, 4096);
        reader
            .read_to_end(&mut decompressed)
            .map_err(|e| e.to_string())?;
        Ok(decompressed)
    }
}
