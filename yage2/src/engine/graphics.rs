use crate::engine::object::Renderable;

pub struct TickResult {
    pub drawn_triangles: usize,
}

pub trait Graphics {
    type Error;
    type InitArgs<'a>;

    fn new(init: Self::InitArgs<'_>) -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn tick(&mut self, renderables: &[Renderable]) -> Result<TickResult, Self::Error>;
}
