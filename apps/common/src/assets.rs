use log::info;
use std::collections::HashMap;
use yage2_core::assets::raw::AssetRaw;
use yage2_core::assets::reader::AssetReader;
use yage2_core::assets::{AssetHeader, AssetID};

fn get_current_exe() -> std::path::PathBuf {
    std::env::current_exe().expect("Failed to get current executable path")
}

pub struct YARCReader {
    filename: String,
}

impl YARCReader {
    pub fn new(filename: String) -> YARCReader {
        YARCReader { filename }
    }
}

impl AssetReader for YARCReader {
    fn read(&mut self) -> Result<HashMap<AssetID, (AssetHeader, AssetRaw)>, String> {
        let assets = yage2_yarc::read(get_current_exe().parent().unwrap().join(&self.filename))
            .map_err(|e| format!("Failed to read assets: {}", e.to_string()))?;

        info!("Loaded {} assets", assets.len());
        let mut result = HashMap::new();
        for (header, raw) in assets {
            result.insert(header.id.clone(), (header, raw));
        }

        Ok(result)
    }
}
