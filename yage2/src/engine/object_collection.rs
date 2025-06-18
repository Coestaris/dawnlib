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

    fn add_object(&mut self, object: ObjectPtr) {
        let events_mask = object.lock().unwrap().event_mask();
        self.alive_objects.push(NumberedObject {
            id: self.current_id,
            object: Arc::clone(&object),
        });

        for event in events_mask {
            self.mapped_objects
                .entry(event)
                .or_default()
                .push(NumberedObject {
                    id: self.current_id,
                    object: Arc::clone(&object),
                });
        }

        self.current_id += 1;
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

    pub(crate) fn dispatch_event(
        &mut self,
        ctx: &ObjectCtx,
        event: &event::Event,
    ) -> DispatchAction {
        let event_kind = event.kind();
        let mut dead_objects: Vec<ObjectPtr> = Vec::new();
        let mut new_objects: Vec<ObjectPtr> = Vec::new();

        if let Some(objs) = self.mapped_objects.get(&event_kind) {
            for obj in objs {
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

        // Remove dead objects and add new objects
        for dead_object in dead_objects {
            self.remove_object(&dead_object);
        }
        for new_object in new_objects {
            self.add_object(new_object);
        }

        // Return an empty action if no special action was requested
        DispatchAction::Empty
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
