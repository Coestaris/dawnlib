mod geometry;
mod input;
mod events;
mod cursor;

use crate::gl::ViewHandleOpenGL;
use crate::input::{InputEvent, MouseButton};
use crate::view::windows::geometry::{SavedWindowState, WindowMode};
use crate::view::windows::input::convert_key;
use crate::view::{TickResult, ViewConfig, ViewCursor, ViewGeometry, ViewTrait};
use crossbeam_channel::Sender;
use log::{debug, info, warn};
use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use thiserror::Error;
use windows::core::{s, w, HSTRING, PCSTR, PCWSTR};
use windows::Win32::Foundation::{
    FreeLibrary, GetLastError, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, WIN32_ERROR, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsW, GetDC, ReleaseDC, CDS_FULLSCREEN, CDS_RESET, DEVMODEW, DM_PELSHEIGHT,
    DM_PELSWIDTH, HDC,
};
use windows::Win32::Graphics::OpenGL::{
    wglCreateContext, wglDeleteContext, wglGetProcAddress, wglMakeCurrent, ChoosePixelFormat,
    SetPixelFormat, SwapBuffers, HGLRC, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW, PFD_SUPPORT_OPENGL,
    PFD_TYPE_RGBA, PIXELFORMATDESCRIPTOR,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;
use windows::Win32::UI::WindowsAndMessaging::*;
use crate::view::windows::events::win_proc;

#[derive(Clone, Debug)]
pub struct PlatformSpecificViewConfig {}

#[derive(Debug, Clone, Error)]
#[allow(dead_code)]
pub enum ViewError {
    #[error("Invalid HWND")]
    InvalidHWND(),
    #[error("Invalid HDC")]
    InvalidHDC,
    #[error("Invalid HINSTANCE")]
    InvalidHINSTANCE(),
    #[error("Invalid class name: {0}")]
    InvalidClassName(String),
    #[error("Failed to get instance handle: {0:?}")]
    GetInstanceError(WIN32_ERROR),
    #[error("Failed to register window class: {0:?}")]
    RegisterClassError(WIN32_ERROR),
    #[error("Failed to create window: {0:?}")]
    CreateWindowError(WIN32_ERROR),
    #[error("Failed to destroy window: {0:?}")]
    SetWindowTextError(WIN32_ERROR),
    #[error("Invalid pixel format")]
    InvalidPixelFormat,
    #[error("Failed to create OpenGL context: {0:?}")]
    ContextCreationError(WIN32_ERROR),
    #[error("Failed to load OpenGL function {1}: {0:?}")]
    FunctionLoadError(WIN32_ERROR, String),
    #[error("Failed to create cursor: {0:?}")]
    CreateCursorError(WIN32_ERROR),
}
const CLASS_NAME: PCWSTR = w!("DAWN_WINDOW_CLASS");

fn to_wide_null(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub(crate) struct View {
    hwnd: HWND,
    hinstance: HINSTANCE,
    cursor: ViewCursor,
    events_sender: Sender<InputEvent>,

    mode: WindowMode,
    saved: Option<SavedWindowState>,
}

impl ViewTrait for View {
    fn open(
        cfg: ViewConfig,
        events_sender: Sender<InputEvent>,
    ) -> Result<Self, crate::view::ViewError> {
        unsafe {
            debug!("Retrieving the instance handle");
            let hinstance = match GetModuleHandleW(None) {
                Ok(handle) => Ok(HINSTANCE::from(handle)),
                Err(_) => Err(ViewError::GetInstanceError(get_last_error())),
            }?;

            match RegisterClassW(&WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                hInstance: hinstance,
                lpszClassName: CLASS_NAME,
                lpfnWndProc: Some(win_proc),
                ..Default::default()
            }) {
                0 => Err(ViewError::RegisterClassError(get_last_error())),
                atom => Ok(atom),
            }?;

            debug!("Creating window");
            let hwnd = match CreateWindowExW(
                WINDOW_EX_STYLE(0),
                CLASS_NAME,
                None,
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                800i32,
                600i32,
                None,
                None,
                hinstance.into(),
                None,
            ) {
                Ok(hwnd) => Ok(hwnd),
                Err(_) => Err(ViewError::CreateWindowError(get_last_error())),
            }?;

            info!("WIN32 Window created successfully");
            let mut view = View {
                hwnd,
                hinstance,
                cursor: ViewCursor::Default,
                events_sender,
                mode: WindowMode::Normal,
                saved: None,
            };

            view.set_title(&cfg.title)?;
            view.set_cursor(cfg.cursor.clone())?;
            view.set_geometry(cfg.geometry.clone())?;
            Ok(view)
        }
    }

    fn get_handle(&self) -> ViewHandle {
        ViewHandle {
            hwnd: self.hwnd,
            hinstance: self.hinstance,
            ctx: None,
            hdc: None,
            opengl32_hmod: None,
        }
    }

    fn tick(&mut self) -> TickResult {
        self.tick_inner()
    }

    fn set_geometry(&mut self, geometry: ViewGeometry) -> Result<(), crate::view::ViewError> {
        self.set_geometry_inner(geometry)
    }

    fn set_title(&mut self, title: &str) -> Result<(), crate::view::ViewError> {
        debug!("Setting window title: {}", title);
        let wide_title = to_wide_null(title);
        unsafe {
            SetWindowTextW(self.hwnd, PCWSTR(wide_title.as_ptr()))
                .map_err(|_| ViewError::SetWindowTextError(get_last_error()))
        }
    }

    fn set_cursor(&mut self, cursor: ViewCursor) -> Result<(), crate::view::ViewError> {
        self.set_cursor_inner(cursor)
    }
}

impl Drop for View {
    fn drop(&mut self) {
        info!("Destroying WIN32 window and releasing resources");
        unsafe {
            if !self.hwnd.is_invalid() {
                DestroyWindow(self.hwnd).ok();
            }
        }
    }
}

#[allow(dead_code)]
pub struct ViewHandle {
    hwnd: HWND,
    hinstance: HINSTANCE,
    ctx: Option<HGLRC>,
    hdc: Option<HDC>,

    opengl32_hmod: Option<HMODULE>,
}

#[cfg(feature = "gl")]
impl ViewHandle {
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

    pub fn error_box(title: &str, message: &str) {
        use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK};

        let title = to_wide_null(title);
        let message = to_wide_null(message);

        unsafe {
            let _ = MessageBoxW(
                None,
                PCWSTR(message.as_ptr()),
                PCWSTR(title.as_ptr()),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}

#[cfg(feature = "gl")]
impl ViewHandleOpenGL for ViewHandle {
    fn create_context(&mut self, _fps: usize, _vsync: bool) -> Result<(), crate::view::ViewError> {
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

            let hdc = GetDC(Some(self.hwnd));
            if hdc.is_invalid() {
                return Err(ViewError::InvalidHDC);
            }

            let pixel_format = ChoosePixelFormat(hdc, &pfd);
            SetPixelFormat(hdc, pixel_format, &pfd).map_err(|_| ViewError::InvalidPixelFormat)?;

            let hglrc = wglCreateContext(hdc)
                .map_err(|_| ViewError::ContextCreationError(get_last_error()))?;
            wglMakeCurrent(hdc, hglrc)
                .map_err(|_| ViewError::ContextCreationError(get_last_error()))?;

            // Set the OpenGL context
            self.hdc = Some(hdc);
            self.ctx = Some(hglrc);

            info!("OpenGL context created successfully");
            Ok(())
        }
    }

    fn get_proc_addr(&mut self, symbol: &str) -> Result<*const c_void, crate::view::ViewError> {
        // debug!("Loading OpenGL function: {}", symbol);

        if self.hdc.is_none() || self.ctx.is_none() {
            return Err(ViewError::InvalidHDC);
        }
        #[cfg(debug_assertions)]
        unsafe {
            use windows::Win32::Graphics::OpenGL::wglGetCurrentContext;
            assert!(
                !wglGetCurrentContext().is_invalid(),
                "No current OpenGL context"
            );
        }

        unsafe {
            self.load_gl_proc(symbol)
                .ok_or_else(|| ViewError::FunctionLoadError(get_last_error(), symbol.to_string()))
        }
    }

    fn swap_buffers(&self) -> Result<(), crate::view::ViewError> {
        if self.hdc.is_none() || self.ctx.is_none() {
            return Err(ViewError::InvalidHDC);
        }

        unsafe {
            if SwapBuffers(self.hdc.unwrap()).is_err() {
                return Err(ViewError::ContextCreationError(get_last_error()));
            }

            Ok(())
        }
    }
}

impl Drop for ViewHandle {
    fn drop(&mut self) {
        info!("Destroying OpenGL context and releasing resources");

        if self.hdc.is_none() || self.ctx.is_none() {
            return;
        }

        let hdc = self.hdc.unwrap();
        let hglrc = self.ctx.unwrap();

        unsafe {
            // Close opengl32.dll module handle if it was loaded
            if let Some(hmod) = self.opengl32_hmod {
                FreeLibrary(hmod).ok();
            }
            // Delete the OpenGL context
            wglDeleteContext(hglrc).ok();
            // Release the device context
            ReleaseDC(Some(self.hwnd), hdc);
        }
    }
}

fn get_last_error() -> WIN32_ERROR {
    unsafe { GetLastError() }
}

