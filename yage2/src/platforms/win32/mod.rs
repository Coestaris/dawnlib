use crate::engine::application::{Application, ApplicationConfig, ApplicationError};
use crate::engine::graphics::Graphics;
use crate::engine::input::{InputEvent, KeyCode, MouseButton};
use crate::engine::vulkan::{VulkanGraphics, VulkanGraphicsError, VulkanGraphicsInitArgs};
use crate::engine::window::{Window, WindowConfig, WindowFactory};
use ash::vk;
use ash::vk::Win32SurfaceCreateInfoKHR;
use log::{debug, info, warn};
use std::ffi::c_char;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{
    GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, WIN32_ERROR, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcA, DestroyWindow, DispatchMessageA, GetMessageA, PostQuitMessage,
    RegisterClassW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, MSG, WINDOW_EX_STYLE, WM_DESTROY,
    WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_PAINT, WM_RBUTTONDOWN, WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPEDWINDOW,
    WS_VISIBLE,
};

#[derive(Debug)]
#[allow(dead_code)]
pub enum Win32Error {
    InvalidHWND(),
    InvalidHINSTANCE(),
    InvalidClassName(String),

    GetInstanceError(WIN32_ERROR),
    RegisterClassError(WIN32_ERROR),
    CreateWindowError(WIN32_ERROR),
    SetWindowTextError(WIN32_ERROR),
    ShowWindowError(WIN32_ERROR),
    UpdateWindowError(WIN32_ERROR),

    GraphicsCreateError(VulkanGraphicsError),
    VulkanCreateSurfaceError(vk::Result),
    VulkanUpdateSurfaceError(VulkanGraphicsError),
}

pub struct Win32Window {
    hwnd: HWND,
    hinstance: HINSTANCE,
    graphics: VulkanGraphics,
    events_sender: Sender<InputEvent>,
}

const CLASS_NAME: &str = "Yage2 Window Class";

pub struct Win32Application {
    window_factory: Win32WindowFactory,
}

enum Message {
    DestroyWindow,
}

impl Application<Win32Error, VulkanGraphics, Win32Window> for Win32Application {
    fn new(config: ApplicationConfig) -> Result<Win32Application, ApplicationError<Win32Error>>
    where
        Self: Sized,
    {
        debug!("Creating Win32 application with config: {:?}", config);
        let window_factory =
            Win32WindowFactory::new(config.window_config).map_err(ApplicationError::InitError)?;
        Ok(Win32Application { window_factory })
    }

    fn get_window_factory(
        &self,
    ) -> Arc<dyn WindowFactory<Win32Window, Win32Error, VulkanGraphics> + Send + Sync> {
        Arc::new(self.window_factory.clone())
    }
}

#[derive(Clone, Debug)]
pub struct Win32WindowFactory {
    config: WindowConfig,
}

impl WindowFactory<Win32Window, Win32Error, VulkanGraphics> for Win32WindowFactory {
    fn new(config: WindowConfig) -> Result<Self, Win32Error>
    where
        Self: Sized,
    {
        debug!("Creating Win32 window factory with config: {:?}", config);
        Ok(Win32WindowFactory { config })
    }

    fn create_window(&self, events_sender: Sender<InputEvent>) -> Result<Win32Window, Win32Error> {
        unsafe {
            debug!("Retrieving the instance handle");
            let hinstance = match GetModuleHandleW(None) {
                Ok(handle) => Ok(HINSTANCE::from(handle)),
                Err(_) => Err(Win32Error::GetInstanceError(get_last_error())),
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
                0 => Err(Win32Error::RegisterClassError(get_last_error())),
                atom => Ok(atom),
            }?;

            debug!(
                "Creating window. w={}, h={}",
                self.config.width, self.config.height
            );
            let title = HSTRING::from("TItle");
            let hwnd = match CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                self.config.width.cast_signed(),
                self.config.height.cast_signed(),
                None,
                None,
                hinstance.into(),
                None,
            ) {
                Ok(hwnd) => Ok(hwnd),
                Err(_) => Err(Win32Error::CreateWindowError(get_last_error())),
            }?;

            debug!("Creating Vulkan graphics");
            let graphics = VulkanGraphics::new(VulkanGraphicsInitArgs {
                instance_extensions: vec![ash::khr::win32_surface::NAME.as_ptr() as *const c_char],
                device_extensions: vec![],
                layers: vec![],
                surface_constructor: Box::new(|entry, instance| {
                    debug!("Creating Win32 Vulkan surface");
                    let surface_loader = ash::khr::win32_surface::Instance::new(entry, instance);
                    let create_info = Win32SurfaceCreateInfoKHR {
                        hinstance: hinstance.0.addr() as _,
                        hwnd: hwnd.0.addr() as _,
                        ..Default::default()
                    };
                    let surface = surface_loader
                        .create_win32_surface(&create_info, None)
                        .map_err(VulkanGraphicsError::SurfaceCreateError)?;

                    Ok(surface)
                }),
            })
            .map_err(Win32Error::GraphicsCreateError)?;

            info!("WIN32 Window with Vulkan graphics created successfully");
            Ok(Win32Window {
                hwnd,
                hinstance,
                graphics,
                events_sender,
            })
        }
    }
}

fn get_last_error() -> WIN32_ERROR {
    unsafe { GetLastError() }
}

/* Global atomic flag to signal the application to stop.
 * This is used to handle the WM_DESTROY message and gracefully exit
 * the application.
 * TODO: Consider using a more sophisticated event loop or message handling system.
 */
static DESTROYED: AtomicBool = AtomicBool::new(false);

fn winkey_to_code(key: u32) -> KeyCode {
    match key {
        0x41 => KeyCode::A,
        0x42 => KeyCode::B,
        0x43 => KeyCode::C,
        _ => KeyCode::A, // Default to A for unsupported keys
    }
}

fn winbutton_to_code(button: u32) -> MouseButton {
    match button {
        0x01 => MouseButton::Left,
        0x02 => MouseButton::Right,
        0x04 => MouseButton::Middle,
        _ => MouseButton::Left, // Default to Left for unsupported buttons
    }
}

impl Window<Win32Error, VulkanGraphics> for Win32Window {
    fn tick(&mut self) -> Result<bool, Win32Error> {
        let mut msg = MSG::default();

        if unsafe { GetMessageA(&mut msg, None, 0, 0).as_bool() } {
            unsafe {
                DispatchMessageA(&msg);
            }

            /* Process the message synchronously
             * to make things simpler */
            let mut event: InputEvent;
            match msg.message {
                WM_KEYDOWN => {
                    event = InputEvent::KeyPress(winkey_to_code(msg.wParam.0 as u32));
                }
                WM_KEYUP => {
                    event = InputEvent::KeyRelease(winkey_to_code(msg.wParam.0 as u32));
                }
                WM_LBUTTONDOWN => {
                    event = InputEvent::MouseButtonPress(winbutton_to_code(msg.wParam.0 as u32));
                }
                WM_LBUTTONUP => {
                    event = InputEvent::MouseButtonRelease(winbutton_to_code(msg.wParam.0 as u32));
                }
                WM_MBUTTONDOWN => {
                    event = InputEvent::MouseButtonPress(winbutton_to_code(msg.wParam.0 as u32));
                }
                WM_MBUTTONUP => {
                    event = InputEvent::MouseButtonRelease(winbutton_to_code(msg.wParam.0 as u32));
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
                    event = InputEvent::MouseButtonPress(winbutton_to_code(msg.wParam.0 as u32));
                }
                WM_RBUTTONUP => {
                    event = InputEvent::MouseButtonRelease(winbutton_to_code(msg.wParam.0 as u32));
                }
                _ => {
                    return Ok(true);
                }
            }

            if let Err(e) = self.events_sender.send(event) {
                warn!("Failed to send event: {:?}", e);
            }
        }

        Ok(!DESTROYED.load(std::sync::atomic::Ordering::Relaxed))
    }

    fn kill(&mut self) -> Result<(), Win32Error> {
        unsafe {
            if !self.hwnd.is_invalid() {
                DestroyWindow(self.hwnd);
            }
        }
        Ok(())
    }

    fn get_graphics(&mut self) -> &mut VulkanGraphics {
        &mut self.graphics
    }
}

unsafe extern "system" fn default_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            WM_PAINT => LRESULT(0),

            WM_DESTROY => {
                debug!("WM_DESTROY received, destroying window");
                DESTROYED.store(true, std::sync::atomic::Ordering::Relaxed);
                PostQuitMessage(0);
                LRESULT(0)
            }

            _ => DefWindowProcA(window, message, wparam, lparam),
        }
    }
}
