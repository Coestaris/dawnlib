use evenio::component::Component;
use evenio::event::GlobalEvent;
use glam::Vec3;

#[derive(GlobalEvent)]
pub struct Tick;
#[derive(Component, Debug)]
pub struct Position(Vec3);
#[derive(Component, Debug)]
pub struct Head {
    pub direction: Vec3,
    pub position: Vec3,
}
