use log::{debug, info};
use std::collections::HashMap;
use yage2_core::assets::reader::{AssetRaw, AssetReader};
use yage2_core::assets::AssetID;

fn get_current_exe() -> std::path::PathBuf {
    std::env::current_exe().expect("Failed to get current executable path")
}

pub struct YARCReader {
    filename: String,
    containers: HashMap<AssetID, yage2_yarc::Container>,
}

impl YARCReader {
    pub fn new(filename: String) -> YARCReader {
        YARCReader {
            filename,
            containers: HashMap::new(),
        }
    }
}

impl AssetReader for YARCReader {
    fn read(&mut self) -> Result<HashMap<AssetID, AssetRaw>, String> {
        self.containers =
            yage2_yarc::read(get_current_exe().parent().unwrap().join(&self.filename))
                .map_err(|e| format!("Failed to read assets: {}", e.to_string()))?;

        info!("Loaded {} assets", self.containers.len());
        for (name, container) in &self.containers {
            debug!(
                "Asset: {} (type {:?}). Size: {} bytes",
                name,
                container.metadata.header.asset_type,
                container.binary.len()
            );
        }

        let mut result = HashMap::new();
        for (name, container) in &self.containers {
            let read = AssetRaw {
                id: name.clone(),
                header: container.metadata.header.clone(),
                metadata: container.metadata.type_specific.clone(),
                data: container.binary.clone(),
            };
            result.insert(name.clone(), read);
        }

        Ok(result)
    }
}
