use crate::compression_backend::compress;
use crate::container::{
    CompressionMode, ContainerError, Record, DAC_MAGIC, DATA_MAGIC, MANIFEST_MAGIC, TOC, TOC_MAGIC,
};
use crate::serialize_backend::serialize;
use crate::{CompressionLevel, Manifest};
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetID;
use log::debug;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

struct Segment {
    magic: u8,
    raw: Vec<u8>,
}

struct DataRaw {
    raw: Vec<u8>,
    compression: CompressionMode,
    id: AssetID,
}

fn ir_to_raw(
    id: AssetID,
    ir: &IRAsset,
    cache_dir: PathBuf,
    compression_level: CompressionLevel,
) -> Result<DataRaw, ContainerError> {
    let raw = serialize(&ir).map_err(|e| ContainerError::SerializationError(e))?;

    // Compress the data and see if it's smaller
    let compressed =
        compress(&raw, compression_level).map_err(|e| ContainerError::CompressionError(e))?;
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

pub fn write_container<W: Write>(
    writer: &mut W,
    manifest: Manifest,
    irs: HashMap<AssetID, IRAsset>,
    cache_dir: PathBuf,
    compression_level: CompressionLevel,
) -> Result<(), ContainerError> {
    let manifest = serialize(&manifest).map_err(|e| ContainerError::SerializationError(e))?;

    // Convert IRs to raw data and compress if possible
    let mut data = Vec::new();
    for (id, ir) in &irs {
        data.push(ir_to_raw(
            id.clone(),
            ir,
            cache_dir.clone(),
            compression_level.clone(),
        )?);
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
