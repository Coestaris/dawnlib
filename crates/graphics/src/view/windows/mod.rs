mod input;

use crate::gl::ViewHandleOpenGL;
use crate::input::{InputEvent, MouseButton};
use crate::view::windows::input::{convert_key, convert_mouse_button};
use crate::view::{TickResult, ViewConfig, ViewTrait};
use log::{debug, info, warn};
use std::ffi::c_void;
use std::sync::Arc;
use crossbeam_channel::Sender;
use windows::core::{s, HSTRING, PCSTR, PCWSTR};
use windows::Win32::Foundation::{
    FreeLibrary, GetLastError, FARPROC, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, WIN32_ERROR,
    WPARAM,
};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC, HDC};
use windows::Win32::Graphics::OpenGL::{
    wglCreateContext, wglDeleteContext, wglGetCurrentContext, wglGetProcAddress, wglMakeCurrent,
    ChoosePixelFormat, SetPixelFormat, SwapBuffers, HGLRC, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW,
    PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA, PIXELFORMATDESCRIPTOR,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcA, DestroyWindow, DispatchMessageA, GetMessageA, PostMessageW,
    PostQuitMessage, RegisterClassW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, MSG, WINDOW_EX_STYLE,
    WM_APP, WM_CLOSE, WM_DESTROY, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_RBUTTONDOWN,
    WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

#[derive(Clone, Debug)]
pub struct PlatformSpecificViewConfig {}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ViewError {
    InvalidHWND(),
    InvalidHINSTANCE(),
    InvalidClassName(String),

    GetInstanceError(WIN32_ERROR),
    RegisterClassError(WIN32_ERROR),
    CreateWindowError(WIN32_ERROR),
    SetWindowTextError(WIN32_ERROR),
    ShowWindowError(WIN32_ERROR),
    UpdateWindowError(WIN32_ERROR),
    InvalidHDC,
    InvalidPixelFormat,
    ContextCreationError(WIN32_ERROR),
    FunctionLoadError(WIN32_ERROR, String),
}

impl std::fmt::Display for ViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViewError::InvalidHWND() => write!(f, "Invalid HWND"),
            ViewError::InvalidHINSTANCE() => write!(f, "Invalid HINSTANCE"),
            ViewError::InvalidClassName(name) => write!(f, "Invalid class name: {}", name),
            ViewError::GetInstanceError(err) => {
                write!(f, "Failed to get instance handle: {:?}", err)
            }
            ViewError::RegisterClassError(err) => {
                write!(f, "Failed to register window class: {:?}", err)
            }
            ViewError::CreateWindowError(err) => write!(f, "Failed to create window: {:?}", err),
            ViewError::SetWindowTextError(err) => {
                write!(f, "Failed to set window title: {:?}", err)
            }
            ViewError::ShowWindowError(err) => write!(f, "Failed to show window: {:?}", err),
            ViewError::UpdateWindowError(err) => write!(f, "Failed to update window: {:?}", err),
            ViewError::InvalidHDC => write!(f, "Invalid HDC"),
            ViewError::InvalidPixelFormat => write!(f, "Invalid pixel format"),
            ViewError::ContextCreationError(err) => {
                write!(f, "Failed to create OpenGL context: {:?}", err)
            }
            ViewError::FunctionLoadError(err, symbol) => {
                write!(f, "Failed to load function '{}': {:?}", symbol, err)
            }
        }
    }
}

impl std::error::Error for ViewError {}

const CLASS_NAME: &str = "DAWN Window Class";
pub const WM_APP_QUIT_REQUESTED: u32 = WM_APP + 1;

pub(crate) struct View {
    hwnd: HWND,
    hinstance: HINSTANCE,
    events_sender: Sender<InputEvent>,
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

            debug!("Registering window class. class_name={}", CLASS_NAME);
            let class_name = HSTRING::from(CLASS_NAME);
            match RegisterClassW(&WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                hInstance: hinstance,
                lpszClassName: PCWSTR(class_name.as_ptr()),
                lpfnWndProc: Some(default_proc),
                ..Default::default()
            }) {
                0 => Err(ViewError::RegisterClassError(get_last_error())),
                atom => Ok(atom),
            }?;

            debug!("Creating window. w={}, h={}", cfg.width, cfg.height);
            let title = HSTRING::from("TItle");
            let hwnd = match CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                cfg.width as i32,
                cfg.height as i32,
                None,
                None,
                hinstance.into(),
                None,
            ) {
                Ok(hwnd) => Ok(hwnd),
                Err(_) => Err(ViewError::CreateWindowError(get_last_error())),
            }?;

            info!("WIN32 Window with Vulkan graphics created successfully");
            Ok(View {
                hwnd,
                hinstance,
                events_sender,
            })
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
        let mut closed = false;
        let mut msg = MSG::default();
        while unsafe { GetMessageA(&mut msg, Some(self.hwnd), 0, 0).0 != 0 } {
            unsafe {
                DispatchMessageA(&msg);
            }

            /* Process the message synchronously
             * to make things simpler */
            let event: InputEvent;
            match msg.message {
                WM_APP_QUIT_REQUESTED => {
                    debug!("WM_APP_QUIT_REQUESTED received, closing the window");
                    closed = true;
                    continue;
                }
                WM_KEYDOWN => {
                    event = InputEvent::KeyPress(convert_key(VIRTUAL_KEY(msg.wParam.0 as u16)));
                }
                WM_KEYUP => {
                    event = InputEvent::KeyRelease(convert_key(VIRTUAL_KEY(msg.wParam.0 as u16)));
                }
                WM_LBUTTONDOWN => {
                    event = InputEvent::MouseButtonPress(MouseButton::Left);
                }
                WM_LBUTTONUP => {
                    event =
                        InputEvent::MouseButtonRelease(MouseButton::Left);
                }
                WM_MBUTTONDOWN => {
                    event = InputEvent::MouseButtonPress(MouseButton::Middle);
                }
                WM_MBUTTONUP => {
                    event =
                        InputEvent::MouseButtonRelease(MouseButton::Middle);
                }
                WM_MOUSEMOVE => {
                    let x = (msg.lParam.0 as i32 & 0xFFFF) as f32;
                    let y = (msg.lParam.0 >> 16) as i32 as f32;
                    event = InputEvent::MouseMove { x, y };
                }
                WM_MOUSEWHEEL => {
                    let delta = (msg.wParam.0 as i32 >> 16) as f32 / 120.0; // Convert to standard scroll units
                    event = InputEvent::MouseScroll {
                        delta_x: 0.0,
                        delta_y: delta,
                    };
                }
                WM_RBUTTONDOWN => {
                    event = InputEvent::MouseButtonPress(MouseButton::Right);
                }
                WM_RBUTTONUP => {
                    event =
                        InputEvent::MouseButtonRelease(MouseButton::Right);
                }
                _ => {
                    return if !closed {
                        TickResult::Continue
                    } else {
                        TickResult::Closed
                    }
                }
            }

            self.events_sender.send(event).unwrap();
        }

        if !closed {
            TickResult::Continue
        } else {
            TickResult::Closed
        }
    }

    fn set_size(&self, width: usize, height: usize) {
        todo!()
    }

    fn set_title(&self, title: &str) {
        todo!()
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
                if let Ok(hmod) = GetModuleHandleA(s!("opengl32.dll")) {
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
}

#[cfg(feature = "gl")]
impl ViewHandleOpenGL for ViewHandle {
    fn create_context(&mut self, fps: usize, vsync: bool) -> Result<(), crate::view::ViewError> {
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

unsafe extern "system" fn default_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_PAINT => LRESULT(0),

        WM_CLOSE => {
            /* Send a custom message to request the application to quit */
            PostMessageW(Some(hwnd), WM_APP_QUIT_REQUESTED, WPARAM(0), LPARAM(0));
            /* Block the message loop until the window is destroyed */
            LRESULT(0)
        }

        WM_DESTROY => {
            debug!("WM_DESTROY received, destroying window");
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }

        _ => unsafe { DefWindowProcA(hwnd, message, wparam, lparam) },
    }
}
