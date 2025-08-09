use log::{debug, info};
use std::collections::HashMap;
use yage2_core::assets::reader::{AssetHeader, AssetReader};
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
    fn has_updates(&self) -> bool {
        true
    }

    fn enumerate(&mut self) -> Result<HashMap<AssetID, AssetHeader>, String> {
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
            result.insert(name.clone(), container.metadata.header.clone());
        }

        Ok(result)
    }

    fn load(&mut self, name: AssetID) -> Result<Vec<u8>, String> {
        if let Some(container) = self.containers.get(&name) {
            debug!("Loading resource: {}", name);
            // TODO: get rid of clone
            Ok(container.binary.clone())
        } else {
            Err(format!("Asset {} not found", name))
        }
    }
}
