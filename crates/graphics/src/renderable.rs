use crate::ecs::{ObjectMaterial, ObjectPointLight, ObjectSunLight};
use crate::gl::material::Material;
use crate::gl::mesh::Mesh;
use dawn_assets::{Asset, TypedAsset};
use evenio::prelude::EntityId;
use glam::{Mat4, Quat, Vec3};
use std::any::TypeId;
use std::ptr::NonNull;
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub struct RenderableUID(EntityId);

#[derive(Clone, Debug)]
pub struct RenderableMeta {
    pub uid: RenderableUID,
    pub updated: bool,
}

impl RenderableMeta {
    pub fn new(uid: EntityId) -> Self {
        RenderableMeta {
            uid: RenderableUID(uid),
            updated: false,
        }
    }
}

impl PartialEq for RenderableMeta {
    fn eq(&self, other: &Self) -> bool {
        self.uid.0 == other.uid.0
    }
}

#[derive(Clone, Debug)]
pub struct RenderablePointLight {
    pub meta: RenderableMeta,
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

impl RenderablePointLight {
    pub fn new(uid: EntityId, object: &ObjectPointLight, position: Vec3) -> Self {
        RenderablePointLight {
            meta: RenderableMeta::new(uid),
            position,
            color: object.color,
            intensity: object.intensity,
        }
    }

    pub fn set_updated(&mut self, updated: bool) {
        self.meta.updated = updated;
    }
}

impl PartialEq for RenderablePointLight {
    fn eq(&self, other: &Self) -> bool {
        self.position.abs_diff_eq(other.position, 0.0001)
            && self.color.abs_diff_eq(other.color, 0.0001)
            && (self.intensity - other.intensity).abs() < 0.0001
            && self.meta == other.meta
    }
}

#[derive(Clone, Debug)]
pub struct RenderableSpotLight {
    pub meta: RenderableMeta,
}

#[derive(Clone, Debug)]
pub struct RenderableSunLight {
    pub meta: RenderableMeta,
    pub intensity: f32,
    pub direction: Vec3,
    pub color: Vec3,
}

impl RenderableSunLight {
    pub fn new(uid: EntityId, object: &ObjectSunLight) -> Self {
        RenderableSunLight {
            meta: RenderableMeta::new(uid),
            intensity: object.intensity,
            direction: object.direction,
            color: object.color,
        }
    }

    pub fn set_updated(&mut self, updated: bool) {
        self.meta.updated = updated;
    }
}

impl PartialEq for RenderableSunLight {
    fn eq(&self, other: &Self) -> bool {
        self.direction.abs_diff_eq(other.direction, 0.0001)
            && self.color.abs_diff_eq(other.color, 0.0001)
            && (self.intensity - other.intensity).abs() < 0.0001
            && self.meta == other.meta
    }
}

#[derive(Clone, Debug)]
pub struct RenderableAreaLight {
    pub meta: RenderableMeta,
}

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
    pub meta: RenderableMeta,
    pub model: Mat4,
    pub material: TypedAsset<Material>,
    pub mesh: TypedAsset<Mesh>,
}

impl Renderable {
    pub fn new(
        uid: EntityId,
        position: Vec3,
        rotation: Quat,
        scale: Vec3,
        material: Option<TypedAsset<Material>>,
        mesh: TypedAsset<Mesh>,
    ) -> Self {
        let model = Mat4::from_scale_rotation_translation(scale, rotation, position);

        Renderable {
            meta: RenderableMeta::new(uid),
            model,
            material: material.unwrap_or_else(ObjectMaterial::default_material),
            mesh,
        }
    }

    pub fn set_updated(&mut self, updated: bool) {
        self.meta.updated = updated;
    }
}

impl PartialEq for Renderable {
    fn eq(&self, other: &Self) -> bool {
        self.model.abs_diff_eq(other.model, 0.0001)
            && self.material == other.material
            && self.mesh == other.mesh
            && self.meta == other.meta
    }
}
