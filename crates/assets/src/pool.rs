use crate::factory::AssetQueryID;
use crate::{Asset, AssetID};
use log::warn;
use std::cell::RefCell;
use std::collections::HashSet;
use crate::registry::AssetRegistry;

pub(crate) struct QueryPool {
    queries: RefCell<Vec<AssetQueryID>>,
}

enum QueryCommand {
    IR,
    Load,
    Free,
}

struct Query {
    id: AssetQueryID,
    asset_id: AssetID,
    dependencies: HashSet<AssetQueryID>,
    command: QueryCommand,
}

impl QueryPool {
    pub fn new() -> Self {
        QueryPool {
            queries: RefCell::new(Vec::new()),
        }
    }

    pub fn query(&self, aid: AssetID, registry: AssetRegistry) -> Result<(), String> {

    }
}
