use crate::raw::AssetRaw;
use crate::{AssetHeader, AssetID};
use std::collections::HashMap;

pub trait AssetReader {
    fn read(&mut self) -> Result<HashMap<AssetID, (AssetHeader, AssetRaw)>, String>;
}
