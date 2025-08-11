use std::ops;

#[repr(C)]
#[derive(Debug, Clone)]
struct InnerResult {
    /// Number of any draw calls made during the pass execution.
    calls: usize,

    /// Number of primitives drawn during the pass execution (e.g., triangles, lines).
    /// This is not the number of vertices, but rather the number of
    /// primitives that were actually rendered.
    primitives: usize,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct PassExecuteResult(Option<InnerResult>);

impl Default for PassExecuteResult {
    #[inline(always)]
    fn default() -> Self {
        PassExecuteResult::ok(0, 0)
    }
}

impl PassExecuteResult {
    #[inline(always)]
    pub fn failed() -> Self {
        PassExecuteResult(None)
    }

    #[inline(always)]
    pub fn ok(calls: usize, primitives: usize) -> Self {
        PassExecuteResult(Some(InnerResult { calls, primitives }))
    }

    #[inline(always)]
    pub fn is_ok(&self) -> bool {
        self.0.is_some()
    }

    #[inline(always)]
    pub fn draw_calls(&self) -> Option<usize> {
        self.0.as_ref().map(|inner| inner.calls)
    }

    #[inline(always)]
    pub fn primitives(&self) -> Option<usize> {
        self.0.as_ref().map(|inner| inner.primitives)
    }
}

impl ops::Add for PassExecuteResult {
    type Output = Self;

    #[inline(always)]
    fn add(self, other: Self) -> Self {
        // If any of the results is None, return None.
        if self.0.is_none() || other.0.is_none() {
            PassExecuteResult(None)
        } else {
            let inner_self = self.0.unwrap();
            let inner_other = other.0.unwrap();
            PassExecuteResult(Some(InnerResult {
                calls: inner_self.calls + inner_other.calls,
                primitives: inner_self.primitives + inner_other.primitives,
            }))
        }
    }
}

impl ops::AddAssign for PassExecuteResult {
    #[inline(always)]
    fn add_assign(&mut self, other: Self) {
        // If any of the results is None, set self to None.
        if self.0.is_none() || other.0.is_none() {
            self.0 = None;
        } else {
            let inner_self = self.0.as_mut().unwrap();
            let inner_other = other.0.unwrap();
            inner_self.calls += inner_other.calls;
            inner_self.primitives += inner_other.primitives;
        }
    }
}
