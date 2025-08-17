use crate::input::InputEvent;
use crate::passes::events::{PassEventTrait, RenderPassEvent};
use crate::renderable::{Material, Position, Renderable, RenderableMesh, Rotation, Scale};
use crate::renderer::monitor::RendererMonitoring;
use crate::renderer::Renderer;
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::{Fetcher, Single};
use evenio::query::Query;
use evenio::world::World;
use glam::Vec3;
use log::info;
use std::ptr::NonNull;
use std::sync::atomic::Ordering;
use dawn_ecs::{StopEventLoop, Tick};

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
        _: Receiver<Tick>,
        renderer: Single<&Boxed>,
        mut sender: Sender<StopEventLoop>,
    ) {
        // Check if the view was closed, if so, send a global event to stop the event loop
        let renderer = renderer.cast::<E>();
        if renderer.stop_signal.load(Ordering::Relaxed) {
            info!("View closed, stopping the event loop");
            sender.send(StopEventLoop);
        }
    }

    // Check if there's any monitor frame to process.
    // If so, push them to the ECS
    fn monitoring_handler<E: PassEventTrait>(
        _: Receiver<Tick>,
        renderer: Single<&Boxed>,
        mut sender: Sender<RendererMonitoring>,
    ) {
        let renderer = renderer.cast::<E>();
        while let Some(frame) = renderer.monitor_queue.pop() {
            sender.send(frame);
        }
    }

    // Check if there's any input event to process.
    // If so, push them to the ECS
    fn inputs_handler<E: PassEventTrait>(
        _: Receiver<Tick>,
        renderer: Single<&Boxed>,
        mut sender: Sender<InputEvent>,
    ) {
        let renderer = renderer.cast::<E>();
        while let Some(input) = renderer.inputs_queue.pop() {
            sender.send(input);
        }
    }

    // Transfer render pass events from the ECS to the renderer thread
    fn render_pass_event_handler<E: PassEventTrait>(
        rpe: Receiver<RenderPassEvent<E>>,
        renderer: Single<&Boxed>,
    ) {
        let renderer = renderer.cast::<E>();
        let _ = renderer.renderer_queue.push(rpe.event.clone());
    }

    #[derive(Query)]
    struct Query<'a> {
        mesh: &'a RenderableMesh,
        position: Option<&'a Position>,
        rotation: Option<&'a Rotation>,
        scale: Option<&'a Scale>,
        material: Option<&'a Material>,
    }

    // Collect renderables from the ECS and send them to the renderer thread
    // This function will be called every tick to collect the renderables
    // and send them to the renderer thread.
    fn collect_renderables<E: PassEventTrait>(
        _: Receiver<Tick>,
        mut renderer: Single<&mut Boxed>,
        fetcher: Fetcher<Query>,
    ) {
        // TODO: Do not allocate a new vector every time, instead use a static one!
        let renderer = renderer.cast_mut::<E>();
        let mut renderables = Vec::new();
        for query in fetcher.iter() {
            // Collect the renderable data from the query
            let mesh_id = query.mesh.mesh_id;
            let position = query.position.map_or(Vec3::ZERO, |p| p.0);
            let rotation = query.rotation.map_or(Vec3::ZERO, |r| r.0);
            let scale = query.scale.map_or(Vec3::ONE, |s| s.0);
            let material = query.material.map_or(Material::default(), |m| m.clone());

            // Create a new Renderable instance
            let renderable = Renderable {
                model: glam::Mat4::from_scale_rotation_translation(
                    scale,
                    glam::Quat::from_euler(glam::EulerRot::XYZ, rotation.x, rotation.y, rotation.z),
                    position,
                ),
                mesh_id,
                material,
            };

            // Push the renderable to the vector
            renderables.push(renderable);
        }

        // Send the collected renderables to the renderer thread
        renderer.renderables_buffer_input.write(renderables);
    }

    world.add_handler(monitoring_handler::<E>);
    world.add_handler(inputs_handler::<E>);
    world.add_handler(view_closed_handler::<E>);
    world.add_handler(collect_renderables::<E>);
    world.add_handler(render_pass_event_handler::<E>);
}
