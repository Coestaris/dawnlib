use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::AssetID;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IRGlyph {
    // The width and height of the glyph in pixels on the atlas.
    pub width: u32,
    pub height: u32,
    // The x and y offset of the glyph on the atlas.
    pub x: u32,
    pub y: u32,
    pub x_advance: u32,
    pub y_offset: i32,
    pub x_offset: i32,
    // The atlas index of the glyph.
    pub page: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IRFont {
    pub glyphs: HashMap<char, IRGlyph>,
    pub  atlases: Vec<AssetID>,
}

impl IRFont {
    pub fn memory_usage(&self) -> usize {
        let mut sum = 0;
        sum += size_of::<IRFont>();
        for atlas in &self.atlases {
            sum += atlas.memory_usage();
        }
        for _ in &self.glyphs {
            sum += size_of::<IRGlyph>();
        }
        sum
    }
}
