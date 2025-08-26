use crate::container::{
    CompressionMode, ContainerError, Record, DAC_MAGIC, DATA_MAGIC, MANIFEST_MAGIC, TOC, TOC_MAGIC,
};
use crate::serialize_backend::serialize;
use crate::Manifest;
use dawn_assets::AssetHeader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;

struct Segment {
    magic: u8,
    raw: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct BinaryAsset {
    #[serde(with = "serde_bytes")]
    pub raw: Vec<u8>,
    pub header: AssetHeader,
    pub compression: CompressionMode,
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
    binaries: Vec<BinaryAsset>,
) -> Result<(), ContainerError> {
    let manifest = serialize(&manifest).map_err(|e| ContainerError::SerializationError(e))?;

    // Create TOC (Table of contents)
    // All the offsets are relative to the start of the data segment
    let mut toc = TOC(HashMap::new());
    let mut offset = 0u32;
    for binary in &binaries {
        toc.0.insert(
            binary.header.id.clone(),
            Record {
                offset,
                length: binary.raw.len() as u32,
                compression: binary.compression,
            },
        );

        offset = offset
            .checked_add(binary.raw.len() as u32)
            .ok_or(ContainerError::SizeOverflow)?;
    }

    // Serialize TOC and data
    let toc = serialize(&toc).map_err(|e| ContainerError::SerializationError(e))?;
    let data = binaries
        .into_iter()
        .flat_map(|d| d.raw)
        .collect::<Vec<u8>>();

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
