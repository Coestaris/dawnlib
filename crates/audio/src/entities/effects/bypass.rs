use crate::entities::{BlockInfo, Effect};
use crate::sample::PlanarBlock;

pub struct BypassEffect {}

impl Effect for BypassEffect {
    fn bypass(&self) -> bool {
        true
    }

    fn render(&mut self, _: &PlanarBlock<f32>, _: &mut PlanarBlock<f32>, _: &BlockInfo) {
        unreachable!()
    }
}

impl BypassEffect {
    pub fn new() -> Self {
        BypassEffect {}
    }
}
