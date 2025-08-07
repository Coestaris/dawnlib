use std::cell::RefCell;
use crate::resources::reader::ResourceHeader;
use crate::resources::resource::ResourceID;
use log::{info, warn};
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use crate::resources::factory::QueryID;

pub(crate) enum ResourceState {
    Raw(Vec<u8>),
    Loaded(NonNull<()>),
    Freed,
}

pub(crate) struct ResourceRegistryItem {
    pub(crate) header: ResourceHeader,
    pub(crate) state: ResourceState,
    pub(crate) in_use: Arc<AtomicBool>,
}

pub(crate) struct ResourcesRegistry(HashMap<ResourceID, ResourceRegistryItem>);

impl ResourcesRegistry {
    pub fn new() -> Self {
        ResourcesRegistry(HashMap::new())
    }

    pub fn push(&mut self, id: ResourceID, raw: Vec<u8>, header: ResourceHeader) {
        info!(
            "Registering resource: {} (type {:?})",
            id, header.resource_type
        );

        let in_use = Arc::new(AtomicBool::new(false));
        let state = ResourceState::Raw(raw);
        self.0.insert(
            id,
            ResourceRegistryItem {
                header,
                state,
                in_use: in_use.clone(),
            },
        );
    }

    pub fn get(&self, id: &ResourceID) -> Option<&ResourceRegistryItem> {
        self.0.get(id)
    }

    pub fn get_mut(&mut self, id: &ResourceID) -> Option<&mut ResourceRegistryItem> {
        self.0.get_mut(id)
    }

    pub fn keys(&self) -> impl Iterator<Item = &ResourceID> {
        self.0.keys()
    }

    pub fn all_loaded(&self) -> bool {
        self.0
            .values()
            .all(|item| matches!(item.state, ResourceState::Loaded(_)))
    }

    pub fn all_freed(&self) -> bool {
        self.0
            .values()
            .all(|item| matches!(item.state, ResourceState::Freed))
    }
}

pub(crate) struct QueriesRegistry {
    queries: RefCell<Vec<QueryID>>
}

impl QueriesRegistry {
    pub fn new() -> Self {
        QueriesRegistry {
            queries: RefCell::new(Vec::new()),
        }
    }

    pub fn add_query(&self, query_id: QueryID) {
        if !self.queries.borrow().contains(&query_id) {
            self.queries.borrow_mut().push(query_id);
        } else {
            warn!("Query {} already exists", query_id);
        }
    }

    pub fn remove_query(&self, query_id: &QueryID) {
        let mut queries = self.queries.borrow_mut();
        if let Some(pos) = queries.iter().position(|q| q == query_id) {
            queries.remove(pos);
        } else {
            warn!("Query {} not found", query_id);
        }
    }
}