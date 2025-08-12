use serde::{Deserialize, Serialize};
use std::time::Instant;
use yage2_core::assets::metadata::{AssetHeader, TypeSpecificMetadata};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AssetMetadata {
    #[serde(default)]
    pub header: AssetHeader,
    #[serde(default)]
    pub type_specific: TypeSpecificMetadata,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Compression {
    None,
    Gzip,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ReadMode {
    Flat,
    Recursive,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ChecksumAlgorithm {
    Md5,
    Blake3,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct WriteOptions {
    pub compression: Compression,
    pub read_mode: ReadMode,
    pub checksum_algorithm: ChecksumAlgorithm,
}

fn serialize_instant<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let duration = instant.elapsed();
    serializer.serialize_u64(duration.as_secs() * 1_000 + u64::from(duration.subsec_millis()))
}

fn deserialize_instant<'de, D>(deserializer: D) -> Result<Instant, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let timestamp = u64::deserialize(deserializer)?;
    Ok(Instant::now() - std::time::Duration::from_millis(timestamp))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub tool_created: String,
    pub tool_version: String,
    #[serde(
        serialize_with = "serialize_instant",
        deserialize_with = "deserialize_instant"
    )]
    pub date_created: Instant,
    pub write_options: WriteOptions,
    pub headers: Vec<AssetHeader>,
}
