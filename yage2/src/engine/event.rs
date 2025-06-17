use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct EventKind: u32 {
        const UPDATE                = 0b00000000; // No events, just an update tick
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
    Unknown(u32),

    Left,
    Right,
    Middle,
    Special(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Unknown(u32, u32),

    Latin(char),
    Cyrillic(char),
    Digit(u8),

    /* Function keys */
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
    Prior,
    PageUp,
    Next,
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
    ScriptSwitch,
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
    QuoteRight,
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
    KPPrior,
    KPPageUp,
    KPNext,
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
    Update(f32), // Delta time in milliseconds
    KeyPress(KeyCode),
    KeyRelease(KeyCode),
    MouseMove { x: f32, y: f32 },
    MouseScroll { delta_x: f32, delta_y: f32 },
    MouseButtonPress(MouseButton),
    MouseButtonRelease(MouseButton),
}

impl Event {
    pub fn kind(&self) -> EventKind {
        match self {
            Event::Update(_) => EventKind::UPDATE,
            Event::KeyPress(_) => EventKind::KEY_PRESS,
            Event::KeyRelease(_) => EventKind::KEY_RELEASE,
            Event::MouseMove { .. } => EventKind::MOUSE_MOVE,
            Event::MouseScroll { .. } => EventKind::MOUSE_SCROLL,
            Event::MouseButtonPress(_) => EventKind::MOUSE_BUTTON_PRESS,
            Event::MouseButtonRelease(_) => EventKind::MOUSE_BUTTON_RELEASE,
        }
    }
}
