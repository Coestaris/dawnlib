use evenio::component::Component;
use glam::{Mat4, Vec3, Vec4};
use dawn_assets::TypedAsset;
use crate::gl::entities::mesh::Mesh;

/// ECS component for specifying the rotation of a renderable object.
/// If entity has no `Rotation` component, it will use the default rotation (0, 0, 0).
/// The rotation is specified in radians around the x, y, and z axes.
#[derive(Component)]
pub struct Rotation(pub(crate) Vec3);

/// ECS component for specifying the position of a renderable object.
/// If entity has no `Position` component, it will use the default position (0, 0, 0).
/// The position is specified in world coordinates.
#[derive(Component)]
pub struct Position(pub Vec3);

/// ECS component for specifying the scale of a renderable object.
#[derive(Component)]
pub struct Scale(pub(crate) Vec3);

/// ECS component for specifying the mesh to be rendered.
/// Also used as a marker to indicate that the entity is renderable.
/// If entity has no `RenderableMesh` component, it will not be rendered.
#[derive(Component)]
pub struct RenderableMesh {
    pub asset: TypedAsset<Mesh>,
}

#[derive(Clone)]
pub struct Renderable {
    pub model: Mat4,
    pub mesh: TypedAsset<Mesh>,
}
