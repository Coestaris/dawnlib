use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct EventMask: u32 {
        const CREATE                = 0b000000001;
        const UPDATE                = 0b000000010;
        const KEY_PRESS             = 0b000000100;
        const KEY_RELEASE           = 0b000001000;
        const CHAR_INPUT            = 0b000010000;
        const MOUSE_MOVE            = 0b000100000;
        const MOUSE_SCROLL          = 0b001000000;
        const MOUSE_BUTTON_PRESS    = 0b010000000;
        const MOUSE_BUTTON_RELEASE  = 0b100000000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Unknown(u32),

    Left,
    Right,
    Middle,
    Special(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Unknown(u32, u32),

    // A-Z keys. Always uppercase.
    Latin(char),
    Cyrillic(char),
    Digit(u8),

    // Function keys
    BackSpace,
    Tab,
    Linefeed,
    Clear,
    Return,
    Pause,
    ScrollLock,
    SysReq,
    Escape,
    Delete,
    Home,
    Left,
    Up,
    Right,
    Down,
    PageUp,
    PageDown,
    End,
    Begin,
    WinL,
    WinR,
    App,
    Select,
    Print,
    Execute,
    Insert,
    Undo,
    Redo,
    Menu,
    Find,
    Cancel,
    Help,
    Break,
    ModeSwitch,
    NumLock,
    Function(u8),
    ShiftL,
    ShiftR,
    ControlL,
    ControlR,
    CapsLock,
    ShiftLock,
    MetaL,
    MetaR,
    AltL,
    AltR,
    SuperL,
    SuperR,
    HyperL,
    HyperR,
    Space,

    /* Printable keys */
    Exclam,
    Quotedbl,
    NumberSign,
    Dollar,
    Percent,
    Ampersand,
    Apostrophe,
    ParenLeft,
    ParenRight,
    Asterisk,
    Plus,
    Comma,
    Minus,
    Period,
    Slash,
    Colon,
    Semicolon,
    Less,
    Equal,
    Greater,
    Question,
    At,
    BracketLeft,
    BracketRight,
    Backslash,
    AsciiCircum,
    Underscore,
    Grave,
    BraceLeft,
    BraceRight,
    Bar,
    Tilde,

    /* Keypad keys */
    KPSpace,
    KPTab,
    KPEnter,
    KPHome,
    KPLeft,
    KPUp,
    KPRight,
    KPDown,
    KPPageUp,
    KPPageDown,
    KPEnd,
    KPBegin,
    KPInsert,
    KPDelete,
    KPEqual,
    KPMultiply,
    KPAdd,
    KPSeparator,
    KPSubtract,
    KPDecimal,
    KPDivide,
    KPDigit(u8),
}

#[derive(Debug, Clone)]
pub enum Event {
    Create, // Object creation event
    Update(f32), // Delta time in milliseconds
    KeyPress(KeyCode),
    KeyRelease(KeyCode),
    CharInput(char),
    MouseMove { x: f32, y: f32 },
    MouseScroll { delta_x: f32, delta_y: f32 },
    MouseButtonPress(MouseButton),
    MouseButtonRelease(MouseButton),
}

impl Event {
    pub fn kind(&self) -> EventMask {
        match self {
            Event::Create => EventMask::CREATE,
            Event::Update(_) => EventMask::UPDATE,
            Event::KeyPress(_) => EventMask::KEY_PRESS,
            Event::KeyRelease(_) => EventMask::KEY_RELEASE,
            Event::MouseMove { .. } => EventMask::MOUSE_MOVE,
            Event::CharInput(_) => EventMask::CHAR_INPUT,
            Event::MouseScroll { .. } => EventMask::MOUSE_SCROLL,
            Event::MouseButtonPress(_) => EventMask::MOUSE_BUTTON_PRESS,
            Event::MouseButtonRelease(_) => EventMask::MOUSE_BUTTON_RELEASE,
        }
    }
}
