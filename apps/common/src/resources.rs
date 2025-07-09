use log::info;
use std::collections::HashMap;
use yage2_core::resources::{ResourceHeader, ResourceManagerIO};

fn get_current_exe() -> std::path::PathBuf {
    std::env::current_exe().expect("Failed to get current executable path")
}

pub struct YARCResourceManagerIO {
    filename: String,
    containers: HashMap<String, yage2_yarc::Container>,
}

impl YARCResourceManagerIO {
    pub fn new(filename: String) -> YARCResourceManagerIO {
        YARCResourceManagerIO {
            filename,
            containers: HashMap::new(),
        }
    }
}

impl ResourceManagerIO for YARCResourceManagerIO {
    fn has_updates(&self) -> bool {
        true
    }

    fn enumerate_resources(&mut self) -> Result<HashMap<String, ResourceHeader>, String> {
        self.containers =
            yage2_yarc::read(get_current_exe().parent().unwrap().join(&self.filename))
                .map_err(|e| format!("Failed to read resources: {}", e.to_string()))?;

        info!("Loaded {} resources", self.containers.len());
        for (name, container) in &self.containers {
            info!(
                "Resource: {} (type {:?}). Size: {} bytes)",
                name,
                container.metadata.header.resource_type,
                container.binary.len()
            );
        }

        let mut result = HashMap::new();
        for (name, container) in &self.containers {
            result.insert(name.clone(), container.metadata.header.clone());
        }

        Ok(result)
    }

    fn load(&mut self, name: String) -> Result<Vec<u8>, String> {
        if let Some(container) = self.containers.get(&name) {
            info!("Loading resource: {}", name);
            // TODO: get rid of clone
            Ok(container.binary.clone())
        } else {
            Err(format!("Resource {} not found", name))
        }
    }
}
