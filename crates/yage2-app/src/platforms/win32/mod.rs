mod input;

use crate::engine::application::{Application, ApplicationConfig, ApplicationError};
use crate::engine::graphics::Graphics;
use crate::engine::input::Event;
use crate::engine::vulkan::{VulkanGraphicsError};
use crate::engine::vulkan::graphics::{VulkanGraphics, VulkanGraphicsInitArgs};
use crate::engine::vulkan::objects::surface::Surface;
use crate::engine::window::{Window, WindowConfig, WindowFactory};
use crate::platforms::win32::input::{convert_key, convert_mouse_button};
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
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;
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
    VulkanCreateSurfaceError(ash::vk::Result),
    VulkanUpdateSurfaceError(VulkanGraphicsError),
}

#[allow(dead_code)]
pub struct Win32Window {
    hwnd: HWND,
    hinstance: HINSTANCE,
    graphics: VulkanGraphics,
    events_sender: Sender<Event>,
}

const CLASS_NAME: &str = "Yage2 Window Class";

pub struct Win32Application {
    window_factory: Win32WindowFactory,
}

impl Application<Win32Window, Win32Error, VulkanGraphics, VulkanGraphicsError>
    for Win32Application
{
    fn new(
        config: ApplicationConfig,
    ) -> Result<Win32Application, ApplicationError<Win32Error, VulkanGraphicsError>>
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

    fn create_window(&self, events_sender: Sender<Event>) -> Result<Win32Window, Win32Error> {
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
                    Surface::new(
                        entry,
                        instance,
                        hinstance.0 as ash::vk::HINSTANCE,
                        hwnd.0 as ash::vk::HWND,
                        Some("win32_surface".to_string())
                    )
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

impl Window<Win32Error, VulkanGraphics> for Win32Window {
    fn tick(&mut self) -> Result<bool, Win32Error> {
        let res = !DESTROYED.load(std::sync::atomic::Ordering::Relaxed);

        let mut msg = MSG::default();
        while unsafe { GetMessageA(&mut msg, Some(self.hwnd), 0, 0).0 != 0 } {
            unsafe {
                DispatchMessageA(&msg);
            }

            /* Process the message synchronously
             * to make things simpler */
            let event: Event;
            match msg.message {
                WM_KEYDOWN => {
                    event = Event::KeyPress(convert_key(VIRTUAL_KEY(msg.wParam.0 as u16)));
                }
                WM_KEYUP => {
                    event = Event::KeyRelease(convert_key(VIRTUAL_KEY(msg.wParam.0 as u16)));
                }
                WM_LBUTTONDOWN => {
                    event = Event::MouseButtonPress(convert_mouse_button(msg.wParam.0 as u32));
                }
                WM_LBUTTONUP => {
                    event = Event::MouseButtonRelease(convert_mouse_button(msg.wParam.0 as u32));
                }
                WM_MBUTTONDOWN => {
                    event = Event::MouseButtonPress(convert_mouse_button(msg.wParam.0 as u32));
                }
                WM_MBUTTONUP => {
                    event = Event::MouseButtonRelease(convert_mouse_button(msg.wParam.0 as u32));
                }
                WM_MOUSEMOVE => {
                    let x = (msg.lParam.0 as i32 & 0xFFFF) as f32;
                    let y = (msg.lParam.0 >> 16) as i32 as f32;
                    event = Event::MouseMove { x, y };
                }
                WM_MOUSEWHEEL => {
                    let delta = (msg.wParam.0 as i32 >> 16) as f32 / 120.0; // Convert to standard scroll units
                    event = Event::MouseScroll {
                        delta_x: 0.0,
                        delta_y: delta,
                    };
                }
                WM_RBUTTONDOWN => {
                    event = Event::MouseButtonPress(convert_mouse_button(msg.wParam.0 as u32));
                }
                WM_RBUTTONUP => {
                    event = Event::MouseButtonRelease(convert_mouse_button(msg.wParam.0 as u32));
                }
                _ => {
                    return Ok(res);
                }
            }

            if let Err(e) = self.events_sender.send(event) {
                warn!("Failed to send event: {:?}", e);
            }
        }

        Ok(res)
    }

    fn kill(&mut self) -> Result<(), Win32Error> {
        if !self.hwnd.is_invalid() {
            // Ignore the result, as we are just cleaning up
            let _ = unsafe { DestroyWindow(self.hwnd) };
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
    match message {
        WM_PAINT => LRESULT(0),

        WM_DESTROY => {
            debug!("WM_DESTROY received, destroying window");
            DESTROYED.store(true, std::sync::atomic::Ordering::Relaxed);
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }

        _ => unsafe { DefWindowProcA(window, message, wparam, lparam) },
    }
}
