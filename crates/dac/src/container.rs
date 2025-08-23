use crate::compression_backend::{compress, decompress};
use crate::manifest::Manifest;
use crate::serialize_backend::{deserialize, serialize};
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetID;
use log::debug;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use thiserror::Error;

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

struct Segment {
    magic: u8,
    raw: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
enum CompressionMode {
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

struct DataRaw {
    raw: Vec<u8>,
    compression: CompressionMode,
    id: AssetID,
}

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

fn ir_to_raw(id: AssetID, ir: &IRAsset) -> Result<DataRaw, ContainerError> {
    let raw = serialize(&ir).map_err(|e| ContainerError::SerializationError(e))?;

    // Compress the data and see if it's smaller
    let compressed = compress(&raw).map_err(|e| ContainerError::CompressionError(e))?;
    Ok(if compressed.len() != 0 && compressed.len() < raw.len() {
        debug!(
            "Asset {} compressed from {} to {} ({:.2}%)",
            id,
            raw.len(),
            compressed.len(),
            (compressed.len() as f32 / raw.len() as f32) * 100.0
        );

        DataRaw {
            raw: compressed,
            compression: CompressionMode::Brotli,
            id,
        }
    } else {
        debug!("Asset {} not compressed ({} bytes)", id, raw.len());
        DataRaw {
            raw,
            compression: CompressionMode::None,
            id,
        }
    })
}

fn write_container_from_segments<W: Write>(
    writer: &mut W,
    segments: Vec<Segment>,
) -> Result<(), ContainerError> {
    // Write DAC magic
    writer.write_all(DAC_MAGIC)?;

    // Write segments
    for segment in segments {
        // Write segment magic
        writer.write_all(segment.magic.to_le_bytes().as_slice())?;

        // Write segment length
        let length = segment.raw.len() as u32;
        writer.write_all(length.to_le_bytes().as_slice())?;

        // Write segment data
        writer.write_all(segment.raw.as_slice())?;
    }

    Ok(())
}

pub(crate) fn write_container<W: Write>(
    writer: &mut W,
    manifest: Manifest,
    irs: HashMap<AssetID, IRAsset>,
) -> Result<(), ContainerError> {
    let manifest = serialize(&manifest).map_err(|e| ContainerError::SerializationError(e))?;

    // Convert IRs to raw data and compress if possible
    let mut data = Vec::new();
    for (id, ir) in &irs {
        data.push(ir_to_raw(id.clone(), ir)?);
    }

    // Create TOC (Table of contents)
    // All the offsets are relative to the start of the data segment
    let mut toc = TOC(HashMap::new());
    let mut offset = 0u32;
    for data in &data {
        toc.0.insert(
            data.id.clone(),
            Record {
                offset,
                length: data.raw.len() as u32,
                compression: data.compression,
            },
        );

        offset = offset
            .checked_add(data.raw.len() as u32)
            .ok_or(ContainerError::SizeOverflow)?;
    }

    // Serialize TOC and data
    let toc = serialize(&toc).map_err(|e| ContainerError::SerializationError(e))?;
    let data = data.into_iter().flat_map(|d| d.raw).collect::<Vec<u8>>();

    write_container_from_segments(
        writer,
        vec![
            Segment {
                magic: TOC_MAGIC,
                raw: toc,
            },
            Segment {
                magic: MANIFEST_MAGIC,
                raw: manifest,
            },
            Segment {
                magic: DATA_MAGIC,
                raw: data,
            },
        ],
    )
}

/// Read segments from a DAC file
/// Returns a map of segment magic to segment offset in the file and length
/// The actual segment data can be read by seeking to the offset and reading the length
fn read_segments<R: Read + Seek>(
    reader: &mut R,
) -> Result<HashMap<u8, (usize, usize)>, ContainerError> {
    // To be sure, seek to the start
    reader.seek(SeekFrom::Start(0))?;

    // Read and verify DAC magic
    let mut magic = [0u8; 3];
    reader.read_exact(&mut magic)?;
    if &magic != DAC_MAGIC {
        return Err(ContainerError::InvalidMagic);
    }

    let mut segments = HashMap::new();
    loop {
        // Read segment magic
        let mut segment_magic = [0u8; 1];
        if let Err(e) = reader.read_exact(&mut segment_magic) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                break; // End of a file
            } else {
                return Err(ContainerError::IOError(e));
            }
        }

        // Read segment length
        let mut length_bytes = [0u8; 4];
        reader.read_exact(&mut length_bytes)?;
        let length = u32::from_le_bytes(length_bytes) as usize;

        // Record the offset of the segment data
        let offset = reader.stream_position()? as usize;
        segments.insert(segment_magic[0], (offset, length));

        // Skip segment data
        reader.seek(SeekFrom::Start(offset as u64 + length as u64))?;
    }

    Ok(segments)
}

fn segment_to_object<R: Read + Seek, T: DeserializeOwned>(
    reader: &mut R,
    segments: &HashMap<u8, (usize, usize)>,
    magic: u8,
) -> Result<T, ContainerError> {
    let (offset, length) = segments
        .get(&magic)
        .ok_or(ContainerError::SegmentNotFound)?;

    reader.seek(SeekFrom::Start(*offset as u64))?;
    let mut segment_bytes = vec![0u8; *length];
    reader.read_exact(&mut segment_bytes)?;

    let object: T =
        deserialize(&segment_bytes).map_err(|e| ContainerError::DeserializationError(e))?;
    Ok(object)
}

pub(crate) fn read_manifest<R: Read + Seek>(reader: &mut R) -> Result<Manifest, ContainerError> {
    let segments = read_segments(reader, MANIFEST_MAGIC)?;
    Ok(segment_to_object(reader, &segments, MANIFEST_MAGIC)?)
}

pub(crate) fn read_ir<R: Read + Seek>(
    reader: &mut R,
    id: AssetID,
) -> Result<IRAsset, ContainerError> {
    // Locate and read the TOC
    let segments = read_segments(reader, DATA_MAGIC)?;
    let toc = segment_to_object::<R, TOC>(reader, &segments, TOC_MAGIC)?;

    // Locate the asset in the TOC
    let record = toc
        .0
        .get(&id)
        .ok_or(ContainerError::AssetNotFound(id.clone()))?;
    let (data_offset, data_length) = segments
        .get(&DATA_MAGIC)
        .ok_or(ContainerError::SegmentNotFound)?;
    let data_offset = data_offset + record.offset as usize;

    // Read the asset data
    let mut data_bytes = vec![0u8; record.length as usize];
    reader.seek(SeekFrom::Start(data_offset as u64))?;
    reader.read_exact(&mut data_bytes)?;

    // Decompress if needed
    let decompressed = match record.compression {
        CompressionMode::None => data_bytes,
        CompressionMode::Brotli => {
            decompress(&data_bytes).map_err(|e| ContainerError::CompressionError(e))?
        }
    };

    // Deserialize the asset
    let asset: IRAsset =
        deserialize(&decompressed).map_err(|e| ContainerError::DeserializationError(e))?;
    Ok(asset)
}
