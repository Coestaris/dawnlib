#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use crate::gl::context::windows::WindowsContext;

use anyhow::anyhow;
use std::ffi;
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

trait ContextImpl {
    fn swap_buffers(&mut self);
    fn load_fn(&mut self, name: &str) -> anyhow::Result<*const ffi::c_void>;
}

pub struct Context {
    inner: Box<dyn ContextImpl>,
}

impl Context {
    pub(crate) fn new(
        raw_window: RawWindowHandle,
        raw_display: RawDisplayHandle,
    ) -> anyhow::Result<Self> {
        Ok(Context {
            inner: match (raw_window, raw_display) {
                #[cfg(target_os = "windows")]
                (RawWindowHandle::Win32(win), RawDisplayHandle::Windows(_)) => {
                    let res = WindowsContext::new(win)?;
                    Box::new(res)
                }
                (_, _) => {
                    // String because anyhow error wants arguments to be Send+Sync
                    // that is not implemented for handles
                    return Err(anyhow!(
                        "Unsupported combination of raw handles: {:?} and {:?}",
                        raw_window,
                        raw_display
                    ))
                }
            },
        })
    }

    pub fn swap_buffers(&mut self) {
        self.inner.swap_buffers();
    }

    pub fn load_fn(&mut self, name: &str) -> anyhow::Result<*const ffi::c_void> {
        self.inner.load_fn(name)
    }
}
