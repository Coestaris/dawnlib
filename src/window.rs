pub trait Window {
    type Error;

    fn new() -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn set_title(&self, title: &str) -> Result<(), Self::Error>;
    fn show(&self) -> Result<(), Self::Error>;
    fn event_loop(&self) -> Result<(), Self::Error>;
}
