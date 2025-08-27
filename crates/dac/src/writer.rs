use crate::serialize_backend::serialize;
use crate::{
    CompressionMode, ContainerError, Manifest, Record, DAC_MAGIC, DATA_MAGIC, MANIFEST_MAGIC, TOC,
    TOC_MAGIC,
};
use dawn_assets::AssetHeader;
use dawn_util::profile::Measure;
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
    let _measure = Measure::new("Write DAC container from segments".to_string());

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

// Having a separate function to write the data segment allows us to avoid
// having to concatenate all binary data into a single Vec<u8> in memory.
// This gives like a 100x speedup on dev profile builds for large containers.
pub fn write_data_segment<W: Write>(
    writer: &mut W,
    total_len: u32,
    binaries: Vec<BinaryAsset>,
) -> Result<(), ContainerError> {
    let _measure = Measure::new("Write DAC data segment".to_string());

    // Write DAC magic
    writer.write_all(DATA_MAGIC.to_le_bytes().as_slice())?;

    // Calculate total length of data segment
    writer.write_all(total_len.to_le_bytes().as_slice())?;

    // Write concatenated binary data
    for binary in binaries {
        writer.write_all(binary.raw.as_slice())?;
    }

    Ok(())
}

pub fn write_container<W: Write>(
    writer: &mut W,
    manifest: Manifest,
    binaries: Vec<BinaryAsset>,
) -> Result<(), ContainerError> {
    let _measure = Measure::new("Write DAC container".to_string());

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

    // Serialize and write control segments
    write_container_from_segments(
        writer,
        vec![
            Segment {
                magic: TOC_MAGIC,
                raw: serialize(&toc).map_err(|e| ContainerError::SerializationError(e))?,
            },
            Segment {
                magic: MANIFEST_MAGIC,
                raw: serialize(&manifest).map_err(|e| ContainerError::SerializationError(e))?,
            },
        ],
    )?;
    // Write data segment
    write_data_segment(writer, offset, binaries)?;
    Ok(())
}
