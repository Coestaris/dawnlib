use std::ops;

/// Represents the result of executing a render pass.
/// It can be used to indicate whether the pass execution was successful or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum PassExecuteResult {
    /// Represents a successful pass execution.
    Ok {
        // Number of any draw calls made during the pass execution.
        calls: usize,
        // Number of primitives drawn during the pass execution (e.g., triangles, lines).
        // This is not the number of vertices, but rather the number of
        // primitives that were actually rendered.
        primitives: usize,
    },

    /// Represents a failed pass execution.
    Failed,
}

impl PassExecuteResult {
    #[inline(always)]
    pub fn ok(calls: usize, primitives: usize) -> Self {
        PassExecuteResult::Ok { calls, primitives }
    }

    #[inline(always)]
    pub fn failed() -> Self {
        PassExecuteResult::Failed
    }
}

impl Default for PassExecuteResult {
    #[inline(always)]
    fn default() -> Self {
        PassExecuteResult::ok(0, 0)
    }
}
impl ops::Add for PassExecuteResult {
    type Output = Self;

    #[inline(always)]
    fn add(self, other: Self) -> Self {
        // If any of the results is None, return None.
        match (self, other) {
            (
                PassExecuteResult::Ok { calls, primitives },
                PassExecuteResult::Ok {
                    calls: oc,
                    primitives: op,
                },
            ) => PassExecuteResult::Ok {
                calls: calls + oc,
                primitives: primitives + op,
            },
            _ => PassExecuteResult::Failed,
        }
    }
}

impl ops::AddAssign for PassExecuteResult {
    #[inline(always)]
    fn add_assign(&mut self, other: Self) {
        let other = other.clone();
        *self = *self + other;
    }
}
