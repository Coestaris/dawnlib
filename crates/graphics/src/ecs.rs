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

#[derive(Component)]
pub struct ObjectColor {
    pub color: Vec3,
}

#[derive(Component)]
pub struct ObjectIntensity {
    pub intensity: f32,
}

/// ECS component for point light
/// Can be rendered only if `ObjectPosition` is also present
/// Also can be modified by `ObjectColor` and `ObjectIntensity` components.
#[derive(Component)]
pub struct ObjectPointLight {
    pub range: f32,
    pub linear_falloff: bool,
    pub shadow: bool,
}

/// ECS component for spot light
/// Can be rendered only if `ObjectPosition` is also present
/// Can be modified by `ObjectColor` and `ObjectIntensity` components.
#[derive(Component)]
pub struct ObjectSpotLight {
    pub direction: Vec3,
    pub range: f32,
    pub inner_cone_angle: f32,
    pub outer_cone_angle: f32,
    pub linear_falloff: bool,
    pub shadow: bool,
}

/// ECS component for the Sun light
/// Can be modified by `ObjectColor` and `ObjectIntensity` components.
/// Direction is specified in world coordinates and should be normalized.
#[derive(Component)]
pub struct ObjectSunLight {
    pub direction: Vec3,
    pub ambient: f32,
    pub shadow: bool,
}

#[derive(Component)]
pub struct ObjectAreaLight {}

#[derive(GlobalEvent)]
pub struct InvalidateRendererCache;
