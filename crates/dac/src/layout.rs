// MAGIC (3 bytes) - DAC
// TOC length (4 bytes, le)
// TOC Magic - 0x0
// Serialized TOC. Offsets in toc are not includes the TOC size. So real file pos is 3 + TOC_LENGTH + offset
// MANIFEST Magic - 0x1
// MANIFEST length (4 bytes, le)
// Serialized manifest
// DATA Length (4 bytes, le)
// DATA Magic - 0x2
// List of IRs

use crate::manifest::Manifest;
use crate::serialize_backend::serialize;
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::mem::transmute;

const DAC_MAGIC: &[u8; 3] = b"DAC";
const TOC_MAGIC: u8 = 0x0;
const MANIFEST_MAGIC: u8 = 0x1;
const DATA_MAGIC: u8 = 0x2;

#[repr(C)]
#[repr(packed)]
struct SegmentHeader {
    length: u32,
    magic: u8,
}

#[derive(Serialize, Deserialize)]
enum CompressionMode {
    None,
    Brotli,
}

#[derive(Serialize, Deserialize)]
struct Record {
    offset: u32,
    compression: CompressionMode,
}

#[derive(Serialize, Deserialize)]
struct TOC(HashMap<AssetID, Record>);

fn create_container(manifest: Manifest, irs: Vec<(AssetID, IRAsset)>) -> Result<(), String> {
    struct DataRaw {
        raw: Vec<u8>,
        compression: CompressionMode,
        id: AssetID,
    }
    let mut data = Vec::new();
    for (id, ir) in &irs {
        let raw = serialize(&ir)?;
        // Try to compress it

        let compressed = Vec::new();
        let compressed_cursor = Cursor::new(compressed);
        let mut writer = brotli::CompressorWriter::new(compressed_cursor, 4096, 11, 22);
        writer.write_all(raw.as_slice()).unwrap();
        writer.flush().unwrap();
        let compressed = writer.into_inner().into_inner();

        data.push(if compressed.len() < raw.len() {
            DataRaw {
                raw: compressed,
                compression: CompressionMode::Brotli,
                id: id.clone(),
            }
        } else {
            DataRaw {
                raw,
                compression: CompressionMode::None,
                id: id.clone(),
            }
        });
    }
    let data_header = SegmentHeader {
        length: data.iter().map(|e| e.raw.len()).sum(),
        magic: DATA_MAGIC,
    };

    let manifest = serialize(&manifest)?;
    let manifest_header = SegmentHeader {
        length: manifest.len() as u32,
        magic: MANIFEST_MAGIC,
    };

    let mut toc = TOC(HashMap::new());
    let mut offset = 0;
    for data_raw in data {
        toc.0.insert(
            data_raw.id,
            Record {
                offset,
                compression: data_raw.compression,
            },
        );
        offset += data_raw.raw.len() as u32
    }
    let toc = serialize(&toc)?;
    let toc_header = SegmentHeader {
        length: toc.len() as u32,
        magic: TOC_MAGIC,
    };

    let data = Vec::new();
    let mut cursor = Cursor::new(data);
    fn write_object:

    cursor.write(DAC_MAGIC).unwrap();
    cursor.write(unsafe { transmute::<u8>(&toc_header
