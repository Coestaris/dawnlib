use crate::ecs::{ObjectPointLight, ObjectSpotLight, ObjectSunLight};
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
    pub linear_falloff: bool,
    pub range: f32,
    pub shadow: bool,
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
            shadow: object.shadow,
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
            && self.shadow == other.shadow
    }
}

#[derive(Clone, Debug)]
pub struct RenderableSpotLight {
    pub meta: RenderableMeta,
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32,
    pub inner_cone_angle: f32,
    pub outer_cone_angle: f32,
    pub shadow: bool,
}

impl RenderableSpotLight {
    pub fn new(
        uid: EntityId,
        object: &ObjectSpotLight,
        intensity: f32,
        position: Vec3,
        color: Vec3,
    ) -> Self {
        RenderableSpotLight {
            meta: RenderableMeta::new(uid),
            position,
            direction: object.direction,
            color,
            intensity,
            range: object.range,
            inner_cone_angle: object.inner_cone_angle,
            outer_cone_angle: object.outer_cone_angle,
            shadow: object.shadow,
        }
    }

    pub fn set_updated(&mut self, updated: bool) {
        self.meta.updated = updated;
    }
}

impl PartialEq for RenderableSpotLight {
    fn eq(&self, other: &Self) -> bool {
        self.position.abs_diff_eq(other.position, 0.0001)
            && self.direction.abs_diff_eq(other.direction, 0.0001)
            && self.color.abs_diff_eq(other.color, 0.0001)
            && (self.intensity - other.intensity).abs() < 0.0001
            && self.range == other.range
            && (self.inner_cone_angle - other.inner_cone_angle).abs() < 0.0001
            && (self.outer_cone_angle - other.outer_cone_angle).abs() < 0.0001
            && self.shadow == other.shadow
            && self.meta == other.meta
    }
}

#[derive(Clone, Debug)]
pub struct RenderableSunLight {
    pub meta: RenderableMeta,
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub ambient: f32,
    pub shadow: bool,
}

impl RenderableSunLight {
    pub fn new(uid: EntityId, object: &ObjectSunLight, color: Vec3, intensity: f32) -> Self {
        RenderableSunLight {
            meta: RenderableMeta::new(uid),
            direction: object.direction,
            color,
            intensity,
            ambient: object.ambient,
            shadow: object.shadow,
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
            && self.shadow == other.shadow
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
