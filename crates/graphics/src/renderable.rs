use crate::gl::entities::material::Material;
use crate::gl::entities::mesh::Mesh;
use dawn_assets::{Asset, TypedAsset};
use evenio::component::Component;
use glam::{Mat4, Quat, Vec3};
use std::any::TypeId;
use std::ptr::NonNull;
use std::sync::OnceLock;

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
pub struct ObjectMaterial(pub TypedAsset<Material>);

impl ObjectMaterial {
    pub fn default_material() -> TypedAsset<Material> {
        const LOCK: OnceLock<TypedAsset<Material>> = OnceLock::new();

        // Use OnceLock to ensure the material is created only once
        let binding = LOCK;
        let material = binding.get_or_init(|| {
            let material = Material::default();
            let ptr = Box::into_raw(Box::new(material));
            let asset = Asset::new(
                TypeId::of::<Material>(),
                NonNull::new(ptr as *mut ()).unwrap(),
            );
            TypedAsset::new(asset)
        });

        material.clone()
    }
}

#[derive(Clone)]
pub struct Renderable {
    pub model: Mat4,
    pub material: TypedAsset<Material>,
    pub mesh: TypedAsset<Mesh>,
}
