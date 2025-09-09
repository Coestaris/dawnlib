use crate::ecs::{
    InvalidateRendererCache, ObjectAreaLight, ObjectColor, ObjectIntensity, ObjectMesh,
    ObjectPointLight, ObjectPosition, ObjectRotation, ObjectScale, ObjectSpotLight, ObjectSunLight,
};
use crate::passes::events::{PassEventTrait, RenderPassEvent};
use crate::renderable::{
    Renderable, RenderableAreaLight, RenderablePointLight, RenderableSpotLight, RenderableSunLight,
};
use crate::renderer::monitor::RendererMonitorEvent;
use crate::renderer::{InputEvent, OutputEvent, RendererProxy};
use dawn_ecs::events::{ExitEvent, InterSyncEvent, TickEvent};
use evenio::component::Component;
use evenio::entity::EntityId;
use evenio::event::{Receiver, Sender};
use evenio::fetch::{Fetcher, Single};
use evenio::handler::IntoHandler;
use evenio::query::Query;
use evenio::world::World;
use glam::{Quat, Vec3};
use log::info;
use std::ptr::NonNull;
use std::sync::atomic::Ordering;

#[derive(Query)]
struct RenderableQuery<'a> {
    entity_id: EntityId,
    mesh: &'a ObjectMesh,
    position: Option<&'a ObjectPosition>,
    rotation: Option<&'a ObjectRotation>,
    scale: Option<&'a ObjectScale>,
}

#[derive(Query)]
struct PointLightQuery<'a> {
    entity_id: EntityId,
    light: &'a ObjectPointLight,
    position: &'a ObjectPosition,
    intensity: Option<&'a ObjectIntensity>,
    color: Option<&'a ObjectColor>,
}

#[derive(Query)]
struct SpotLightQuery<'a> {
    entity_id: EntityId,
    light: &'a ObjectSpotLight,
    intensity: Option<&'a ObjectIntensity>,
    color: Option<&'a ObjectColor>,
}

#[derive(Query)]
struct AreaLightQuery<'a> {
    entity_id: EntityId,
    light: &'a ObjectAreaLight,
}

#[derive(Query)]
struct SunLightQuery<'a> {
    entity_id: EntityId,
    light: &'a ObjectSunLight,
    intensity: Option<&'a ObjectIntensity>,
    color: Option<&'a ObjectColor>,
}

#[derive(Component)]
struct Boxed {
    raw: NonNull<()>,
}

impl Boxed {
    fn new<E: PassEventTrait>(renderer: RendererProxy<E>) -> Self {
        let raw = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(renderer)) as *mut ()) };
        Boxed { raw }
    }

    fn cast<E: PassEventTrait>(&self) -> &RendererProxy<E> {
        // SAFETY: We are guaranteed that the raw pointer is valid
        // and points to a RendererProxy<E> because we created it from a Box<RendererProxy<E>>.
        unsafe { &*(self.raw.as_ptr() as *const RendererProxy<E>) }
    }

    fn cast_mut<E: PassEventTrait>(&mut self) -> &mut RendererProxy<E> {
        // SAFETY: We are guaranteed that the raw pointer is valid
        // and points to a RendererProxy<E> because we created it from a Box<RendererProxy<E>>.
        unsafe { &mut *(self.raw.as_ptr() as *mut RendererProxy<E>) }
    }
}

impl Drop for Boxed {
    fn drop(&mut self) {
        info!("Dropping renderer box");

        // TODO: Empty is not an E. Can this break something?
        #[derive(Copy, Clone)]
        struct Empty {}

        unsafe {
            let _ = Box::from_raw(self.raw.as_ptr() as *mut RendererProxy<Empty>);
        };
    }
}

#[derive(Component)]
struct UpdateTracker {
    renderables_cache: foldhash::HashMap<EntityId, Renderable>,
    point_lights_cache: foldhash::HashMap<EntityId, RenderablePointLight>,
    sun_lights_cache: foldhash::HashMap<EntityId, RenderableSunLight>,
    area_lights_cache: foldhash::HashMap<EntityId, RenderableAreaLight>,
    spot_lights_cache: foldhash::HashMap<EntityId, RenderableSpotLight>,
}

impl UpdateTracker {
    fn new() -> Self {
        UpdateTracker {
            renderables_cache: Default::default(),
            point_lights_cache: Default::default(),
            sun_lights_cache: Default::default(),
            area_lights_cache: Default::default(),
            spot_lights_cache: Default::default(),
        }
    }

    fn clear(&mut self) {
        self.renderables_cache.clear();
        self.point_lights_cache.clear();
        self.sun_lights_cache.clear();
        self.area_lights_cache.clear();
        self.spot_lights_cache.clear();
    }

    fn is_updated<T: PartialEq + Clone>(
        entity_id: EntityId,
        cache: &mut foldhash::HashMap<EntityId, T>,
        item: &T,
    ) -> bool {
        match cache.get(&entity_id) {
            Some(cached_item) => {
                if item.eq(cached_item) {
                    false
                } else {
                    cache.insert(entity_id, item.clone());
                    true
                }
            }
            None => {
                cache.insert(entity_id, item.clone());
                true
            }
        }
    }

    fn track_renderable(&mut self, entity_id: EntityId, renderable: &Renderable) -> bool {
        UpdateTracker::is_updated(entity_id, &mut self.renderables_cache, renderable)
    }

    fn track_point_light(
        &mut self,
        entity_id: EntityId,
        point_light: &RenderablePointLight,
    ) -> bool {
        UpdateTracker::is_updated(entity_id, &mut self.point_lights_cache, point_light)
    }

    fn track_sun_light(&mut self, entity_id: EntityId, sun_light: &RenderableSunLight) -> bool {
        UpdateTracker::is_updated(entity_id, &mut self.sun_lights_cache, sun_light)
    }
}

pub fn attach_to_ecs<E: PassEventTrait>(renderer: RendererProxy<E>, world: &mut World) {
    // Setup the renderer player entity in the ECS
    let boxed = world.spawn();
    world.insert(boxed, Boxed::new(renderer));

    let tracker = world.spawn();
    world.insert(tracker, UpdateTracker::new());

    fn invalidate_cache_handler<E: PassEventTrait>(
        _: Receiver<InvalidateRendererCache>,
        mut renderer: Single<&mut Boxed>,
        mut tracker: Single<&mut UpdateTracker>,
    ) {
        info!("Invalidating renderer cache");

        let renderer = renderer.cast_mut::<E>();

        // Clear the tracker
        tracker.clear();

        // Flush the triple buffer... three times
        let frame = renderer.data_stream.input_buffer_mut();
        frame.clear();
        renderer.data_stream.publish();
        let frame = renderer.data_stream.input_buffer_mut();
        frame.clear();
        renderer.data_stream.publish();
        let frame = renderer.data_stream.input_buffer_mut();
        frame.clear();
        renderer.data_stream.publish();
    }

    // If the renderer loop is closed or stopped,
    // we need to stop the event loop
    fn view_closed_handler<E: PassEventTrait>(
        _: Receiver<TickEvent>,
        renderer: Single<&Boxed>,
        mut sender: Sender<ExitEvent>,
    ) {
        // Check if the view was closed, if so, send a global event to stop the event loop
        let renderer = renderer.cast::<E>();
        if renderer.stop_signal.load(Ordering::Relaxed) {
            info!("View closed, stopping the event loop");
            sender.send(ExitEvent);
        }
    }

    // Check if there's any monitor frame to process.
    // If so, push them to the ECS
    fn monitoring_handler<E: PassEventTrait>(
        _: Receiver<TickEvent>,
        renderer: Single<&Boxed>,
        mut sender: Sender<RendererMonitorEvent>,
    ) {
        let renderer = renderer.cast::<E>();
        for frame in renderer.monitor_receiver.try_iter() {
            sender.send(frame);
        }
    }

    // Check if there's any input event to process.
    // If so, push them to the ECS
    fn inputs_handler<E: PassEventTrait>(
        _: Receiver<TickEvent>,
        renderer: Single<&Boxed>,
        mut sender: Sender<InputEvent>,
    ) {
        let renderer = renderer.cast::<E>();
        for event in renderer.input_receiver.try_iter() {
            sender.send(event);
        }
    }

    // Transfer render pass events from the ECS to the renderer thread
    fn render_pass_event_handler<E: PassEventTrait>(
        rpe: Receiver<RenderPassEvent<E>>,
        renderer: Single<&Boxed>,
    ) {
        let renderer = renderer.cast::<E>();
        renderer.renderer_sender.send(rpe.event.clone()).unwrap();
    }

    // Collect renderables from the ECS and send them to the renderer thread
    // This function will be called every tick to collect the renderables
    // and send them to the renderer thread.
    //
    // Ideally this should be done AFTER the main loop, but before the renderer
    // thread is started. Like so (one frame):
    // ╔═══════╤════════════════════════════╤═══════════════════╗
    // ║  ...  │ (Peek data)  [ Rendering ] │              ...  ║ RendererProxy
    // ║       │      [ Processing ]        │ (Post data)       ║ Main Thread
    // ╟───────┼────────────────────────────┼───────────────────╣
    // ║     [Sync]                       [Sync]                ║
    // ╚════════════════════════════════════════════════════════╝
    //
    // This is only possible if the renderer is synchronized with the main thread in two
    // points. If you want the smooth movement of the object, consider using a hard sync
    // instead of free running the renderer thread.
    fn stream_data_handle<E: PassEventTrait>(
        t: Receiver<InterSyncEvent>,
        mut boxed: Single<&mut Boxed>,
        mut tracker: Single<&mut UpdateTracker>,
        renderables: Fetcher<RenderableQuery>,
        point_lights: Fetcher<PointLightQuery>,
        _spot_lights: Fetcher<SpotLightQuery>,
        _area_lights: Fetcher<AreaLightQuery>,
        sun_lights: Fetcher<SunLightQuery>,
    ) {
        let renderer = boxed.cast_mut::<E>();

        // Update the buffer in-place
        let frame = renderer.data_stream.input_buffer_mut();
        frame.clear();
        frame.epoch = t.event.frame;

        // Process the renderables
        for renderable in renderables.iter() {
            // Collect the renderable data from the query
            let mesh_asset = renderable.mesh.0.clone();
            let position = renderable.position.map_or(Vec3::ZERO, |p| p.0);
            let rotation = renderable.rotation.map_or(Quat::IDENTITY, |r| r.0);
            let scale = renderable.scale.map_or(Vec3::ONE, |s| s.0);

            // Push the renderable to the vector
            let mut object =
                Renderable::new(renderable.entity_id, position, rotation, scale, mesh_asset);
            object.set_updated(tracker.track_renderable(renderable.entity_id, &object));
            frame.renderables.push(object);
        }

        // Process the point lights
        for light in point_lights.iter() {
            let position = light.position.0;
            let inner_light = light.light;
            let intensity = light.intensity.map_or(1.0, |i| i.intensity);
            let color = light.color.map_or(Vec3::ONE, |c| c.color);

            let mut object =
                RenderablePointLight::new(light.entity_id, inner_light, color, intensity, position);
            object.set_updated(tracker.track_point_light(light.entity_id, &object));
            frame.point_lights.push(object);
        }

        // Process the sun lights
        for light in sun_lights.iter() {
            let inner_light = light.light;

            let mut object = RenderableSunLight::new(light.entity_id, inner_light);
            object.set_updated(tracker.track_sun_light(light.entity_id, &object));
            frame.sun_lights.push(object);
        }

        // Send the collected renderables to the renderer thread
        renderer.data_stream.publish();
    }

    fn view_handler<E: PassEventTrait>(r: Receiver<OutputEvent>, mut renderer: Single<&mut Boxed>) {
        let renderer = renderer.cast_mut::<E>();
        renderer.output_sender.send(r.event.clone()).unwrap();
    }

    world.add_handler(invalidate_cache_handler::<E>);
    world.add_handler(view_handler::<E>);
    world.add_handler(monitoring_handler::<E>.low());
    world.add_handler(inputs_handler::<E>.high());
    world.add_handler(view_closed_handler::<E>.low());
    world.add_handler(stream_data_handle::<E>);
    world.add_handler(render_pass_event_handler::<E>.high());
}
