use crate::gl::mesh::Mesh;
use dawn_assets::TypedAsset;
use evenio::component::Component;
use evenio::event::GlobalEvent;
use glam::{Quat, Vec3};

/// ECS component for specifying the rotation of a renderable object.
/// If entity has no `Rotation` component, it will use the default rotation (0, 0, 0).
/// The rotation is specified in radians around the x, y, and z axes.
#[derive(Component)]
pub struct ObjectRotation(pub Quat);

/// ECS component for specifying the position of a renderable object.
/// If entity has no `Position` component, it will use the default position (0, 0, 0).
/// The position is specified in world coordinates.
#[derive(Component)]
pub struct ObjectPosition(pub Vec3);

/// ECS component for specifying the scale of a renderable object.
#[derive(Component)]
pub struct ObjectScale(pub Vec3);

/// ECS component for specifying the mesh to be rendered.
/// Also used as a marker to indicate that the entity is renderable.
/// If entity has no `RenderableMesh` component, it will not be rendered.
#[derive(Component)]
pub struct ObjectMesh(pub TypedAsset<Mesh>);

/// ECS component for point light
/// Can be rendered only if `ObjectPosition` is also present
#[derive(Component)]
pub struct ObjectPointLight {
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32,
}

#[derive(Component)]
pub struct ObjectSpotLight {}

/// ECS component for the Sun light
#[derive(Component)]
pub struct ObjectSunLight {
    pub color: Vec3,
    pub intensity: f32,
    pub direction: Vec3,
}

#[derive(Component)]
pub struct ObjectAreaLight {}

#[derive(GlobalEvent)]
pub struct InvalidateRendererCache;
