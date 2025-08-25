use evenio::event::GlobalEvent;

/// Event sent every tick in the main loop (usually 60 times per second).
/// Can be used to update game logic, render frames, etc.
/// It should not be sent by the user.
#[derive(GlobalEvent)]
pub struct TickEvent {
    /// The current frame number.
    pub frame: usize,
    /// The time since the last tick in seconds in milliseconds.
    pub delta: f32,
    /// The total time since the start of the main loop in milliseconds.
    pub time: f32,
}

/// This is a special Tick sent in between frames
/// (between `after_frame` and `before_frame` synchronization).
/// If running unsynchronized, it is almost the same as `Tick`
/// and has no special meaning.
/// It should not be sent by the user.
#[derive(GlobalEvent)]
pub struct InterSyncEvent {
    pub frame: usize,
}

/// Event sent to stop the main loop.
#[derive(GlobalEvent)]
pub struct ExitEvent;
