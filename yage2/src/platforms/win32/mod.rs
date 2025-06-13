use crate::engine::application::{Application, ApplicationConfig, ApplicationError};
use crate::engine::graphics::Graphics;
use crate::engine::vulkan::{VulkanGraphics, VulkanGraphicsError, VulkanGraphicsInitArgs};
use crate::engine::window::{Window, WindowConfig, WindowFactory};
use ash::vk;
use ash::vk::Win32SurfaceCreateInfoKHR;
use log::{debug, info};
use std::ffi::c_char;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{
    GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, WIN32_ERROR, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcA, DispatchMessageA, GetMessageA, PostQuitMessage,
    RegisterClassW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, MSG, WINDOW_EX_STYLE, WM_DESTROY,
    WM_PAINT, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
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
        info!("Creating Win32 application with config: {:?}", config);
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
        info!("Creating Win32 window factory with config: {:?}", config);
        Ok(Win32WindowFactory { config })
    }

    fn create_window(&self) -> Result<Win32Window, Win32Error> {
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
        let mut msg = MSG::default();

        unsafe {
            if GetMessageA(&mut msg, None, 0, 0).as_bool() {
                DispatchMessageA(&msg);
            }
        }

        Ok(!DESTROYED.load(std::sync::atomic::Ordering::Relaxed))
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
