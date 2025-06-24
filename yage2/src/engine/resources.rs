type ResourceId = u32;
type TagId = u32;

trait Resource {
    /// Returns the name of the resource.
    fn name(&self) -> String;
    /// Returns the ID of the resource.
    fn id(&self) -> ResourceId;
    /// Returns the tag ID of the resource, if any.
    fn tag(&self) -> Option<TagId>;
    
    
    /// Marks the resource as out of date (OOD).
    fn mark_ood(&mut self);
    /// Checks if the resource is out of date (OOD).
    fn update(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}