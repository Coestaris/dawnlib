use crate::{AssetHeader, AssetID};
use std::collections::HashMap;
use crate::ir::IRAsset;

pub trait AssetReader {
    fn read(&mut self) -> Result<HashMap<AssetID, (AssetHeader, IRAsset)>, String>;
}
