use crate::ir::IRAsset;
use crate::{Asset, AssetHeader, AssetID, AssetMemoryUsage};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone)]
pub(crate) enum AssetState {
    Empty,
    Read(IRAsset),
    Loaded(Asset, AssetMemoryUsage),
}

#[derive(Error, Debug, Clone)]
pub enum RegistryError {
    #[error("Asset with ID {0} not found")]
    NotFound(AssetID),
}

pub(crate) struct AssetContainer {
    pub(crate) header: AssetHeader,
    pub(crate) state: AssetState,
}

pub(crate) struct AssetRegistry(HashMap<AssetID, AssetContainer>);

impl AssetRegistry {
    pub fn new() -> Self {
        AssetRegistry(HashMap::new())
    }

    pub fn enumerate(&mut self, headers: Vec<AssetHeader>) {
        self.0.clear();
        for header in headers {
            self.0.insert(
                header.id.clone(),
                AssetContainer {
                    header,
                    state: AssetState::Empty,
                },
            );
        }
    }

    pub fn update(&mut self, id: AssetID, state: AssetState) -> Result<(), RegistryError> {
        if let Some(container) = self.0.get_mut(&id) {
            container.state = state;
            Ok(())
        } else {
            Err(RegistryError::NotFound(id))
        }
    }

    pub fn get_header(&self, id: &AssetID) -> Result<&AssetHeader, RegistryError> {
        self.0
            .get(id)
            .map(|container| &container.header)
            .ok_or_else(|| RegistryError::NotFound(id.clone()))
    }

    pub fn get_state(&self, id: &AssetID) -> Result<&AssetState, RegistryError> {
        self.0
            .get(id)
            .map(|container| &container.state)
            .ok_or_else(|| RegistryError::NotFound(id.clone()))
    }

    pub fn keys(&self) -> impl Iterator<Item = &AssetID> {
        self.0.keys()
    }
}
