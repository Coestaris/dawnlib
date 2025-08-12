pub use crate::assets::metadata::{AssetHeader, TypeSpecificMetadata};
use crate::assets::AssetID;
use std::collections::HashMap;

pub struct AssetRaw {
    pub id: AssetID,
    pub header: AssetHeader,
    pub metadata: TypeSpecificMetadata,
    pub data: Vec<u8>,
}

pub trait AssetReader {
    fn read(&mut self) -> Result<HashMap<AssetID, AssetRaw>, String>;
}
