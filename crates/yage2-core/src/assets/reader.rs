use crate::assets::raw::AssetRaw;
use crate::assets::{AssetHeader, AssetID};
use std::collections::HashMap;

pub trait AssetReader {
    fn read(&mut self) -> Result<HashMap<AssetID, (AssetHeader, AssetRaw)>, String>;
}
