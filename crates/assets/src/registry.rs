use std::collections::HashMap;
use crate::ir::IRAsset;
use crate::{Asset, AssetHeader, AssetID, AssetMemoryUsage};
use log::info;

#[derive(Debug, Clone)]
pub(crate) enum AssetState {
    Empty,
    IR(IRAsset),
    Loaded(Asset, AssetMemoryUsage),
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

    pub fn register(&mut self, id: AssetID, header: AssetHeader) {
        info!("Registering asset: {} (type {:?})", id, header.asset_type);

        self.0.insert(
            id,
            AssetContainer {
                header,
                state: AssetState::Empty,
            },
        );
    }

    pub fn update(&mut self, id: AssetID, state: AssetState) -> Result<(), String> {
        if let Some(container) = self.0.get_mut(&id) {
            container.state = state;
            Ok(())
        } else {
            Err(format!("Asset with ID {} not found", id))
        }
    }

    pub fn get_header(&self, id: &AssetID) -> Result<&AssetHeader, String> {
        self.0
            .get(id)
            .map(|container| &container.header)
            .ok_or_else(|| format!("Asset with ID {} not found", id))
    }

    pub fn get_state(&self, id: &AssetID) -> Result<&AssetState, String> {
        self.0
            .get(id)
            .map(|container| &container.state)
            .ok_or_else(|| format!("Asset with ID {} not found", id))
    }

    pub fn keys(&self) -> impl Iterator<Item = &AssetID> {
        self.0.keys()
    }
}
