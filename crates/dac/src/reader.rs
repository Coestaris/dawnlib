use crate::compression_backend::decompress;
use crate::serialize_backend::deserialize;
use crate::{
    CompressionMode, ContainerError, Manifest, DAC_MAGIC, DATA_MAGIC, MANIFEST_MAGIC, TOC,
    TOC_MAGIC,
};
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetID;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};

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

pub fn read_manifest<R: Read + Seek>(reader: &mut R) -> Result<Manifest, ContainerError> {
    let segments = read_segments(reader)?;
    Ok(segment_to_object(reader, &segments, MANIFEST_MAGIC)?)
}

pub fn read_asset<R: Read + Seek>(reader: &mut R, id: AssetID) -> Result<IRAsset, ContainerError> {
    // Locate and read the TOC
    let segments = read_segments(reader)?;
    let toc = segment_to_object::<R, TOC>(reader, &segments, TOC_MAGIC)?;

    // Locate the asset in the TOC
    let record = toc
        .0
        .get(&id)
        .ok_or(ContainerError::AssetNotFound(id.clone()))?;
    let (data_offset, _) = segments
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
