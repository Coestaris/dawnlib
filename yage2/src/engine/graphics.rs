pub trait Graphics {
    type Error;
    type InitArgs<'a>;

    fn new(init: Self::InitArgs<'_>) -> Result<Self, Self::Error>
    where
        Self: Sized;
    
    fn tick(&mut self) -> Result<(), Self::Error>;
}
