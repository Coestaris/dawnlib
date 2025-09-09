use crate::ecs::{ObjectColor, ObjectIntensity, ObjectPointLight, ObjectSunLight};
use crate::gl::mesh::Mesh;
use dawn_assets::TypedAsset;
use evenio::prelude::EntityId;
use glam::{Mat4, Quat, Vec3};

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
    pub range: f32,
    pub linear_falloff: bool,
}

impl RenderablePointLight {
    pub fn new(
        uid: EntityId,
        object: &ObjectPointLight,
        color: Vec3,
        intensity: f32,
        position: Vec3,
    ) -> Self {
        RenderablePointLight {
            meta: RenderableMeta::new(uid),
            position,
            color,
            intensity,
            range: object.range,
            linear_falloff: object.linear_falloff,
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
            && self.linear_falloff == other.linear_falloff
            && self.meta == other.meta
            && self.range == other.range
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

#[derive(Clone)]
pub struct Renderable {
    pub meta: RenderableMeta,
    pub model: Mat4,
    pub mesh: TypedAsset<Mesh>,
}

impl Renderable {
    pub fn new(
        uid: EntityId,
        position: Vec3,
        rotation: Quat,
        scale: Vec3,
        mesh: TypedAsset<Mesh>,
    ) -> Self {
        let model = Mat4::from_scale_rotation_translation(scale, rotation, position);

        Renderable {
            meta: RenderableMeta::new(uid),
            model,
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
            && self.mesh == other.mesh
            && self.meta == other.meta
    }
}
