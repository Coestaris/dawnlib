use crate::assets::factory::AssetQueryID;
use crate::assets::metadata::{AssetHeader, TypeSpecificMetadata};
use crate::assets::reader::AssetRaw;
use crate::assets::AssetID;
use log::{info, warn};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

pub(crate) enum AssetState {
    Raw(Vec<u8>),
    Loaded(TypeId, NonNull<()>),
    Freed,
}

pub(crate) struct AssetContainer {
    pub(crate) header: AssetHeader,
    pub(crate) metadata: TypeSpecificMetadata,
    pub(crate) state: AssetState,
    pub(crate) rc: Arc<AtomicUsize>,
}

pub(crate) struct AssetRegistry(HashMap<AssetID, AssetContainer>);

impl AssetRegistry {
    pub fn new() -> Self {
        AssetRegistry(HashMap::new())
    }

    pub fn push(&mut self, id: AssetID, asset_read: AssetRaw) {
        info!(
            "Registering asset: {} (type {:?})",
            id, asset_read.header.asset_type
        );

        let state = AssetState::Raw(asset_read.data);
        self.0.insert(
            id,
            AssetContainer {
                header: asset_read.header,
                metadata: asset_read.metadata,
                state,
                rc: Arc::new(AtomicUsize::new(0)),
            },
        );
    }

    pub fn get(&self, id: &AssetID) -> Option<&AssetContainer> {
        self.0.get(id)
    }

    pub fn get_mut(&mut self, id: &AssetID) -> Option<&mut AssetContainer> {
        self.0.get_mut(id)
    }

    pub fn keys(&self) -> impl Iterator<Item = &AssetID> {
        self.0.keys()
    }

    pub fn all_loaded(&self) -> bool {
        self.0
            .values()
            .all(|item| matches!(item.state, AssetState::Loaded(_, _)))
    }

    pub fn all_freed(&self) -> bool {
        self.0
            .values()
            .all(|item| matches!(item.state, AssetState::Freed))
    }
}

pub(crate) struct QueriesRegistry {
    queries: RefCell<Vec<AssetQueryID>>,
}

impl QueriesRegistry {
    pub fn new() -> Self {
        QueriesRegistry {
            queries: RefCell::new(Vec::new()),
        }
    }

    pub fn add_query(&self, query_id: AssetQueryID) {
        if !self.queries.borrow().contains(&query_id) {
            self.queries.borrow_mut().push(query_id);
        } else {
            warn!("Query {} already exists", query_id);
        }
    }

    pub fn remove_query(&self, query_id: &AssetQueryID) {
        let mut queries = self.queries.borrow_mut();
        if let Some(pos) = queries.iter().position(|q| q == query_id) {
            queries.remove(pos);
        } else {
            warn!("Query {} not found", query_id);
        }
    }
}
