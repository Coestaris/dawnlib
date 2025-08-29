use std::ops;

/// Represents the result of executing a render pass.
/// It can be used to indicate whether the pass execution was successful or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum RenderResult {
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

impl RenderResult {
    #[inline(always)]
    pub fn ok(calls: usize, primitives: usize) -> Self {
        RenderResult::Ok { calls, primitives }
    }

    #[inline(always)]
    pub fn failed() -> Self {
        RenderResult::Failed
    }
}

impl Default for RenderResult {
    #[inline(always)]
    fn default() -> Self {
        RenderResult::ok(0, 0)
    }
}
impl ops::Add for RenderResult {
    type Output = Self;

    #[inline(always)]
    fn add(self, other: Self) -> Self {
        // If any of the results is None, return None.
        match (self, other) {
            (
                RenderResult::Ok { calls, primitives },
                RenderResult::Ok {
                    calls: oc,
                    primitives: op,
                },
            ) => RenderResult::Ok {
                calls: calls + oc,
                primitives: primitives + op,
            },
            _ => RenderResult::Failed,
        }
    }
}

impl ops::AddAssign for RenderResult {
    #[inline(always)]
    fn add_assign(&mut self, other: Self) {
        let other = other.clone();
        *self = *self + other;
    }
}
