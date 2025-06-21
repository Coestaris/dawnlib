use crate::engine::event;
use crate::engine::event::EventMask;
use crate::engine::object::{DispatchAction, ObjectCtx, ObjectPtr, Renderable};
use std::collections::HashMap;
use std::sync::Arc;

type ObjectID = usize;

pub(crate) struct NumberedObject {
    id: ObjectID,
    object: ObjectPtr,
}

pub(crate) struct ObjectsCollection {
    current_id: ObjectID,

    /// While adding an object to the collection,
    /// the object is mapped to the events it is interested in.
    /// This allows the engine to dispatch events to the correct objects.
    mapped_objects: HashMap<EventMask, Vec<NumberedObject>>,

    /// The collection of objects that are currently alive.
    /// This is used to track the objects that are currently active in the application.
    alive_objects: Vec<NumberedObject>,

    /// The collection of renderable objects.
    /// This is used to render the objects in the application.
    renderables: HashMap<ObjectID, Renderable>,

    renderables_updated: bool,
}

impl ObjectsCollection {
    pub(crate) fn new(objects: Vec<ObjectPtr>) -> Self {
        let mut collection = ObjectsCollection {
            current_id: 0,
            mapped_objects: HashMap::new(),
            alive_objects: Vec::new(),
            renderables: HashMap::new(),
            renderables_updated: false,
        };
        for object in objects {
            collection.add_object(object);
        }
        collection
    }

    fn add_object(&mut self, object: ObjectPtr) -> ObjectID {
        let id = self.current_id;
        self.alive_objects.push(NumberedObject {
            id,
            object: Arc::clone(&object),
        });

        let events_mask = object.lock().unwrap().event_mask();
        for event in events_mask {
            self.mapped_objects
                .entry(event)
                .or_default()
                .push(NumberedObject {
                    id,
                    object: Arc::clone(&object),
                });
        }

        self.current_id = id + 1;
        id
    }

    fn remove_object(&mut self, object: &ObjectPtr) {
        // Remove from alive_objects
        self.alive_objects
            .retain(|obj| !Arc::ptr_eq(&obj.object, object));

        // Remove from mapped_objects
        for objs in self.mapped_objects.values_mut() {
            objs.retain(|obj| !Arc::ptr_eq(&obj.object, object));
        }
    }

    /// Dispatches an event to all objects that are interested in it.
    fn dispatch_event_inner(
        &mut self,
        ctx: &ObjectCtx,
        event: &event::Event,
        objects_whitelist: Option<&[ObjectID]>,
        dead_objects: &mut Vec<ObjectPtr>,
        new_objects: &mut Vec<ObjectPtr>,
    ) -> DispatchAction {
        let event_kind = event.kind();
        if let Some(objs) = self.mapped_objects.get(&event_kind) {
            for obj in objs {
                if let Some(whitelist) = objects_whitelist {
                    if !whitelist.contains(&obj.id) {
                        continue; // Skip objects not in the whitelist
                    }
                }

                if let Ok(mut locked_obj) = obj.object.lock() {
                    match locked_obj.dispatch(ctx, event) {
                        DispatchAction::Die => {
                            dead_objects.push(Arc::clone(&obj.object));
                        }

                        DispatchAction::SpawnObjects(objects) => {
                            new_objects.extend(objects);
                        }

                        DispatchAction::SpawnObject(new_obj) => {
                            new_objects.push(new_obj);
                        }

                        DispatchAction::KillObject(target) => {
                            dead_objects.push(Arc::clone(&target));
                        }

                        DispatchAction::KillObjects(targets) => {
                            for target in targets {
                                dead_objects.push(Arc::clone(&target));
                            }
                        }

                        DispatchAction::QuitApplication => {
                            return DispatchAction::QuitApplication;
                        }

                        DispatchAction::UpdateRenderable(renderable) => {
                            self.renderables.insert(obj.id, renderable);
                            self.renderables_updated = true;
                        }

                        DispatchAction::DeleteRenderable => {
                            self.renderables.remove(&obj.id);
                            self.renderables_updated = true;
                        }

                        DispatchAction::Empty => {}
                    }
                }
            }
        }

        DispatchAction::Empty
    }

    pub(crate) fn dispatch_event(
        &mut self,
        ctx: &ObjectCtx,
        event: &event::Event,
    ) -> Vec<DispatchAction> {
        let mut event = event;
        let mut whitelist = Vec::new();
        let mut result = Vec::new();

        loop {
            let mut dead_objects: Vec<ObjectPtr> = Vec::new();
            let mut new_objects: Vec<ObjectPtr> = Vec::new();

            // Dispatch the event to all objects that are interested in it
            result.push(self.dispatch_event_inner(
                ctx,
                event,
                if whitelist.is_empty() {
                    None
                } else {
                    Some(&whitelist)
                },
                &mut dead_objects,
                &mut new_objects,
            ));

            // Remove dead objects and add new objects
            for dead_object in dead_objects {
                self.remove_object(&dead_object);
            }

            if !new_objects.is_empty() {
                whitelist.clear();
                for new_object in new_objects {
                    whitelist.push(self.add_object(new_object));
                }

                // If new objects were added,
                // we need to re-dispatch the creation event
                event = &event::Event::Create;
            } else {
                // If no new objects were added, we can break the loop
                break;
            }
        }

        // Return an empty action if no special action was requested
        result
    }

    pub(crate) fn alive_objects(&self) -> &Vec<NumberedObject> {
        &self.alive_objects
    }

    pub(crate) fn updated_renderables(&mut self) -> Option<&HashMap<ObjectID, Renderable>> {
        if self.renderables_updated {
            self.renderables_updated = false;
            Some(&self.renderables)
        } else {
            None
        }
    }
}
