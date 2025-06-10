pub trait Graphics {
    type Error;
    type InitArgs;

    fn new(init: Self::InitArgs) -> Result<Self, Self::Error>
    where
        Self: Sized;
}
