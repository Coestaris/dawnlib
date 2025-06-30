use crate::engine::object::Renderable;

pub struct TickResult {
    pub drawn_triangles: usize,
}

pub trait Graphics<GraphicsError> {
    type InitArgs<'a>;

    fn new(init: Self::InitArgs<'_>) -> Result<Self, GraphicsError>
    where
        Self: Sized;

    fn tick(&mut self, renderables: &[Renderable]) -> Result<TickResult, GraphicsError>;
}
