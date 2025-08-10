use std::ops;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct PassExecuteResult {
    /// Number of any draw calls made during the pass execution.
    calls: usize,

    /// Number of primitives drawn during the pass execution (e.g., triangles, lines).
    /// This is not the number of vertices, but rather the number of
    /// primitives that were actually rendered.
    primitives: usize,
}

impl Default for PassExecuteResult {
    #[inline(always)]
    fn default() -> Self {
        PassExecuteResult::new(0, 0)
    }
}

impl PassExecuteResult {
    #[inline(always)]
    pub fn new(calls: usize, primitives: usize) -> Self {
        PassExecuteResult { calls, primitives }
    }
    
    #[inline(always)]
    pub fn draw_calls(&self) -> usize {
        self.calls
    }
    
    #[inline(always)]
    pub fn primitives(&self) -> usize {
        self.primitives
    }
}

impl ops::Add for PassExecuteResult {
    type Output = Self;

    #[inline(always)]
    fn add(self, other: Self) -> Self {
        PassExecuteResult {
            calls: self.calls + other.calls,
            primitives: self.primitives + other.primitives,
        }
    }
}

impl ops::AddAssign for PassExecuteResult {
    #[inline(always)]
    fn add_assign(&mut self, other: Self) {
        self.calls += other.calls;
        self.primitives += other.primitives;
    }
}
