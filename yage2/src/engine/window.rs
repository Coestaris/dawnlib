pub trait Window {
    type Error;

    fn new(title: &str, width: u32, height: u32) -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn event_loop(&self) -> Result<(), Self::Error>;
}
