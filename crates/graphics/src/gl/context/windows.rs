use crate::gl::context::ContextImpl;
use log::{debug, info, warn};
use std::ffi::c_void;
use thiserror::Error;
use windows::core::{w, PCSTR};
use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::Graphics::Gdi::{GetDC, HDC};
use windows::Win32::Graphics::OpenGL::{
    wglCreateContext, wglDeleteContext, wglGetProcAddress, wglMakeCurrent, ChoosePixelFormat,
    SetPixelFormat, SwapBuffers, HGLRC, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW, PFD_SUPPORT_OPENGL,
    PFD_TYPE_RGBA, PIXELFORMATDESCRIPTOR,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use winit::raw_window_handle::{RawWindowHandle, Win32WindowHandle};

pub(super) struct WindowsContext {
    handle: Win32WindowHandle,
    ctx: Option<HGLRC>,
    hdc: Option<HDC>,
    opengl32_hmod: Option<HMODULE>,
}

#[derive(Debug, Clone, Error)]
pub(super) enum WindowsContextError {
    #[error("Invalid HDC")]
    InvalidHDC,
    #[error("Failed to set pixel format: {0}")]
    InvalidPixelFormat(windows::core::Error),
    #[error("Failed to create OpenGL context: {0}")]
    ContextCreationError(windows::core::Error),
    #[error("Failed to make OpenGL context current: {0}")]
    MakeCurrentError(windows::core::Error),
}

impl WindowsContext {
    unsafe fn load_gl_proc(&mut self, symbol: &str) -> Option<*const c_void> {
        unsafe {
            // Convert the symbol to a C-style string
            let c = std::ffi::CString::new(symbol).ok()?;

            // 1) Trying to get a pointer via wglGetProcAddress (fore >1.1 and extensions)
            let p = wglGetProcAddress(PCSTR(c.as_ptr() as _)).map(|f| f as *const c_void);

            // wglGetProcAddress can return NULL and some other values
            // that are not valid function pointers.
            if let Some(ptr) = p {
                let iv = [1usize, 2, 3, usize::MAX];
                if !iv.contains(&(ptr as usize)) {
                    return Some(ptr);
                }
            }

            // 2) Fallback to GetProcAddress
            // This is needed for OpenGL 1.0 and some core functions.
            // It will return NULL if the function is not found.
            self.opengl32_hmod = if let Some(hmod) = self.opengl32_hmod {
                Some(hmod)
            } else {
                if let Ok(hmod) = GetModuleHandleW(w!("opengl32.dll")) {
                    debug!("Loaded opengl32.dll module handle");
                    Some(hmod)
                } else {
                    // If we can't get the module handle, return None
                    warn!("Failed to get module handle for opengl32.dll");
                    return None;
                }
            };

            let p2 = GetProcAddress(self.opengl32_hmod.unwrap(), PCSTR(c.as_ptr() as _))?;
            Some(p2 as *const c_void)
        }
    }

    pub fn new(handle: Win32WindowHandle) -> Result<Self, WindowsContextError> {
        info!("Creating WindowsContext with handle: {:?}", handle);

        let mut context = WindowsContext {
            handle,
            ctx: None,
            hdc: None,
            opengl32_hmod: None,
        };

        unsafe {
            let pfd = PIXELFORMATDESCRIPTOR {
                nSize: size_of::<PIXELFORMATDESCRIPTOR>() as u16,
                nVersion: 1,
                dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
                iPixelType: PFD_TYPE_RGBA,
                cColorBits: 32,
                cDepthBits: 24,
                cStencilBits: 8,
                ..Default::default()
            };

            let hdc = GetDC(Some(HWND(handle.hwnd.get() as *mut core::ffi::c_void)));
            if hdc.is_invalid() {
                return Err(WindowsContextError::InvalidHDC);
            }

            let pixel_format = ChoosePixelFormat(hdc, &pfd);
            SetPixelFormat(hdc, pixel_format, &pfd)
                .map_err(WindowsContextError::InvalidPixelFormat)?;

            // Bootstrap OpenGL context to load functions
            // This context will be replaced later if possible
            let hglrc = wglCreateContext(hdc).map_err(WindowsContextError::ContextCreationError)?;
            wglMakeCurrent(hdc, hglrc).map_err(WindowsContextError::ContextCreationError)?;

            // Try to locate the *Attrib function to create a modern OpenGL context
            // If it fails, we will use the old context created above
            // This is not ideal, but at least it will work on older systems
            // and we can still use modern OpenGL if the function is available
            // Note: wglCreateContextAttribsARB is an extension function, so we need to load it manually
            let hglrc = if let Some(wglCreateContextAttribsARB) =
                context.load_gl_proc("wglCreateContextAttribsARB")
            {
                type WglCreateContextAttribsARB =
                    unsafe extern "system" fn(HDC, HGLRC, *const i32) -> HGLRC;
                let wglCreateContextAttribsARB: WglCreateContextAttribsARB =
                    std::mem::transmute(wglCreateContextAttribsARB);

                // Request an OpenGL 3.3 core profile context
                // God forgive me for this magic number soup
                // See https://www.khronos.org/registry/OpenGL/extensions/ARB/WGL_ARB_create_context.txt
                // for details.
                // TODO: Maybe this constant already exists somewhere in the windows crate?
                const WGL_CONTEXT_MAJOR_VERSION_ARB: i32 = 0x2091;
                const WGL_CONTEXT_MINOR_VERSION_ARB: i32 = 0x2092;
                const WGL_CONTEXT_LAYER_PLANE_ARB: i32 = 0x2093;
                const WGL_CONTEXT_FLAGS_ARB: i32 = 0x2094;
                const WGL_CONTEXT_PROFILE_MASK_ARB: i32 = 0x9126;

                const WGL_CONTEXT_CORE_PROFILE_BIT_ARB: i32 = 0x00000001;

                let attribs = [
                    WGL_CONTEXT_MAJOR_VERSION_ARB,
                    3,
                    WGL_CONTEXT_MINOR_VERSION_ARB,
                    2,
                    WGL_CONTEXT_PROFILE_MASK_ARB,
                    WGL_CONTEXT_CORE_PROFILE_BIT_ARB,
                    0,
                ];

                let ctx = wglCreateContextAttribsARB(hdc, HGLRC::default(), attribs.as_ptr());
                if ctx.is_invalid() {
                    warn!("wglCreateContextAttribsARB failed, falling back to wglCreateContext");
                    hglrc
                } else {
                    // Make the new context current and delete the old one
                    wglMakeCurrent(hdc, ctx).map_err(WindowsContextError::MakeCurrentError)?;
                    // Delete the old context
                    wglDeleteContext(hglrc).ok();
                    info!("Created OpenGL 3.2+ context successfully");
                    ctx
                }
            } else {
                warn!("wglCreateContextAttribsARB not found, using legacy context");
                hglrc
            };

            // Set the OpenGL context
            context.hdc = Some(hdc);
            context.ctx = Some(hglrc);
        }

        info!("OpenGL context created successfully");

        Ok(context)
    }
}

impl Drop for WindowsContext {
    fn drop(&mut self) {
        // Clean up resources if necessary
        info!("Dropping WindowsContext with handle: {:?}", self.handle);
        if let (Some(hdc), Some(ctx)) = (self.hdc, self.ctx) {
            unsafe {
                wglMakeCurrent(hdc, HGLRC::default()).ok();
                wglDeleteContext(ctx).ok();
                // Note: We do not release the HDC obtained via GetDC,
                // as per MSDN it is not necessary and may lead to issues.
            }
            info!("OpenGL context and HDC released");
        }
    }
}

impl ContextImpl for WindowsContext {
    fn swap_buffers(&mut self) {
        if let Some(hdc) = self.hdc {
            unsafe {
                SwapBuffers(hdc).unwrap();
            }
        }
    }

    fn load_fn(&mut self, name: &str) -> anyhow::Result<*const c_void> {
        unsafe {
            if let Some(ptr) = self.load_gl_proc(name) {
                Ok(ptr)
            } else {
                Err(anyhow::anyhow!("Failed to load OpenGL function: {}", name))
            }
        }
    }
}
