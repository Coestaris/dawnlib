use crate::factory::AssetQueryID;
use crate::ir::IRAsset;
use crate::{Asset, AssetHeader, AssetID};
use log::{info, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

pub(crate) enum AssetState {
    Empty,
    IR(IRAsset),
    Loaded(Asset),
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

    pub fn get_header(&self, id: &AssetID) -> Option<&AssetHeader> {
        self.0.get(id).map(|container| &container.header)
    }

    pub fn get_state(&self, id: &AssetID) -> Option<&AssetState> {
        self.0.get(id).map(|container| &container.state)
    }

    pub fn keys(&self) -> impl Iterator<Item = &AssetID> {
        self.0.keys()
    }

    pub fn all_loaded(&self) -> bool {
        self.0
            .values()
            .all(|item| matches!(item.state, AssetState::Loaded(_)))
    }

    pub fn all_empty(&self) -> bool {
        self.0
            .values()
            .all(|item| matches!(item.state, AssetState::Empty))
    }
}