use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use dawn_assets::AssetID;

pub(crate) mod reader;
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

const DAC_MAGIC: &[u8; 3] = b"DAC";
const TOC_MAGIC: u8 = 0x0;
const MANIFEST_MAGIC: u8 = 0x1;
const DATA_MAGIC: u8 = 0x2;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum CompressionMode {
    None,
    Brotli,
}

#[derive(Serialize, Deserialize)]
struct Record {
    offset: u32,
    length: u32,
    compression: CompressionMode,
}

#[derive(Serialize, Deserialize)]
struct TOC(HashMap<AssetID, Record>);

#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Compression error: {0}")]
    CompressionError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
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
    DeserializationError(String),
}
