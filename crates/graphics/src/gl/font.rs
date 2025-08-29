use crate::passes::events::PassEventTrait;
use dawn_assets::ir::font::IRFont;
use dawn_assets::{Asset, AssetID, AssetMemoryUsage};
use log::debug;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FontError {}

pub struct Font {}

impl Font {
    pub(crate) fn from_ir<E: PassEventTrait>(
        ir: IRFont,
        deps: HashMap<AssetID, Asset>,
    ) -> Result<(Self, AssetMemoryUsage), FontError> {
        debug!("Creating Font from IR: {:?}", ir);

        Ok((Font {}, AssetMemoryUsage::new(0, 0)))
    }
}
