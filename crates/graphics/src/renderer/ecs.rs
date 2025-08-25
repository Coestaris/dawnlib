use crate::input::InputEvent;
use crate::passes::events::{PassEventTrait, RenderPassEvent};
use crate::renderable::{
    ObjectMaterial, ObjectMesh, ObjectPosition, ObjectRotation, ObjectScale, Renderable,
};
use crate::renderer::monitor::RendererMonitorEvent;
use crate::renderer::Renderer;
use dawn_ecs::events::{InterSyncEvent, ExitEvent, TickEvent};
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::{Fetcher, Single};
use evenio::handler::IntoHandler;
use evenio::query::Query;
use evenio::world::World;
use glam::{Mat4, Quat, Vec3};
use log::info;
use std::ptr::NonNull;
use std::sync::atomic::Ordering;

pub fn attach_to_ecs<E: PassEventTrait>(renderer: Renderer<E>, world: &mut World) {
    #[derive(Component)]
    struct Boxed {
        raw: NonNull<()>,
    }

    impl Boxed {
        fn new<E: PassEventTrait>(renderer: Renderer<E>) -> Self {
            let raw =
                unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(renderer)) as *mut ()) };
            Boxed { raw }
        }

        fn cast<E: PassEventTrait>(&self) -> &Renderer<E> {
            // SAFETY: We are guaranteed that the raw pointer is valid
            // and points to a Renderer<E> because we created it from a Box<Renderer<E>>.
            unsafe { &*(self.raw.as_ptr() as *const Renderer<E>) }
        }

        fn cast_mut<E: PassEventTrait>(&mut self) -> &mut Renderer<E> {
            // SAFETY: We are guaranteed that the raw pointer is valid
            // and points to a Renderer<E> because we created it from a Box<Renderer<E>>.
            unsafe { &mut *(self.raw.as_ptr() as *mut Renderer<E>) }
        }
    }

    impl Drop for Boxed {
        fn drop(&mut self) {
            info!("Dropping renderer box");

            // TODO: Empty is not an E. Can this break something?
            #[derive(Copy, Clone)]
            struct Empty {}

            unsafe {
                let _ = Box::from_raw(self.raw.as_ptr() as *mut Renderer<Empty>);
            };
        }
    }

    // Setup the renderer player entity in the ECS
    let renderer_entity = world.spawn();
    world.insert(renderer_entity, Boxed::new(renderer));

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
        for event in renderer.inputs_receiver.try_iter() {
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

    #[derive(Query)]
    struct Query<'a> {
        mesh: &'a ObjectMesh,
        position: Option<&'a ObjectPosition>,
        rotation: Option<&'a ObjectRotation>,
        scale: Option<&'a ObjectScale>,
        material: Option<&'a ObjectMaterial>,
    }

    // Collect renderables from the ECS and send them to the renderer thread
    // This function will be called every tick to collect the renderables
    // and send them to the renderer thread.
    //
    // Ideally this should be done AFTER the main loop, but before the renderer
    // thread is started. Like so (one frame):
    // ╔═══════╤════════════════════════════╤═══════════════════╗
    // ║  ...  │ (Peek data)  [ Rendering ] │              ...  ║ Renderer
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
        mut renderer: Single<&mut Boxed>,
        fetcher: Fetcher<Query>,
    ) {
        let renderer = renderer.cast_mut::<E>();

        // Update the renderables buffer in-place
        let frame = renderer.data_stream.input_buffer_mut();

        frame.renderables.clear();
        for query in fetcher.iter() {
            // Collect the renderable data from the query
            let mesh_asset = query.mesh.0.clone();
            let position = query.position.map_or(Vec3::ZERO, |p| p.0);
            let rotation = query.rotation.map_or(Quat::IDENTITY, |r| r.0);
            let scale = query.scale.map_or(Vec3::ONE, |s| s.0);
            let material = query
                .material
                .map_or_else(|| ObjectMaterial::default_material(), |m| m.0.clone());

            // Push the renderable to the vector
            frame.renderables.push(Renderable {
                model: Mat4::from_scale_rotation_translation(scale, rotation, position),
                material,
                mesh: mesh_asset,
            });
        }
        frame.epoch = t.event.frame;

        // Send the collected renderables to the renderer thread
        renderer.data_stream.publish();
    }

    world.add_handler(monitoring_handler::<E>.low());
    world.add_handler(inputs_handler::<E>.high());
    world.add_handler(view_closed_handler::<E>.low());
    world.add_handler(stream_data_handle::<E>);
    world.add_handler(render_pass_event_handler::<E>.high());
}
