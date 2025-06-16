use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct InputEventKind: u32 {
        const KEY_PRESS             = 0b00000001;
        const KEY_RELEASE           = 0b00000010;
        const MOUSE_MOVE            = 0b00000100;
        const MOUSE_SCROLL          = 0b00001000;
        const MOUSE_BUTTON_PRESS    = 0b00010000;
        const MOUSE_BUTTON_RELEASE  = 0b00100000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    // Add more keys as needed
}



#[derive(Debug, Clone)]
pub enum InputEvent {
    KeyPress(KeyCode),
    KeyRelease(KeyCode),
    MouseMove { x: f32, y: f32 },
    MouseScroll { delta_x: f32, delta_y: f32 },
    MouseButtonPress(MouseButton),
    MouseButtonRelease(MouseButton),
}

impl InputEvent {
    pub fn kind(&self) -> InputEventKind {
        match self {
            InputEvent::KeyPress(_) => InputEventKind::KEY_PRESS,
            InputEvent::KeyRelease(_) => InputEventKind::KEY_RELEASE,
            InputEvent::MouseMove { .. } => InputEventKind::MOUSE_MOVE,
            InputEvent::MouseScroll { .. } => InputEventKind::MOUSE_SCROLL,
            InputEvent::MouseButtonPress(_) => InputEventKind::MOUSE_BUTTON_PRESS,
            InputEvent::MouseButtonRelease(_) => InputEventKind::MOUSE_BUTTON_RELEASE,
        }
    }
}