use log::{debug, info};
use std::ffi::c_char;

use crate::graphics::graphics::Graphics;
use crate::graphics::vulkan::{
    VulkanGraphics, VulkanGraphicsError, VulkanGraphicsInitArgs, VulkanGraphicsInternal,
};
use crate::graphics::window::Window;

use ash::vk;
use ash::vk::Win32SurfaceCreateInfoKHR;

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
    InvalidHWND(HWND),
    InvalidHINSTANCE(HINSTANCE),
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

fn get_last_error() -> WIN32_ERROR {
    unsafe { GetLastError() }
}

impl Window for Win32Window {
    type Error = Win32Error;

    fn new(title: &str, width: u32, height: u32) -> Result<Self, Self::Error> {
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

            debug!("Creating window. w={}, h={}", width, height);
            let title = HSTRING::from(title);
            let hwnd = match CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width.cast_signed(),
                height.cast_signed(),
                None,
                None,
                hinstance.into(),
                None,
            ) {
                Ok(hwnd) => Ok(hwnd),
                Err(_) => Err(Win32Error::CreateWindowError(get_last_error())),
            }?;

            debug!("Creating Vulkan graphics");
            let mut graphics = VulkanGraphics::new(VulkanGraphicsInitArgs {
                extensions: vec![ash::khr::win32_surface::NAME.as_ptr() as *const c_char],
                layers: vec![],
            })
            .map_err(Win32Error::GraphicsCreateError)?;

            debug!("Creating Win32 Vulkan surface");
            let create_info = Win32SurfaceCreateInfoKHR {
                hinstance: hinstance.0.addr() as _,
                hwnd: hwnd.0.addr() as _,
                ..Default::default()
            };
            let surface_loader =
                ash::khr::win32_surface::Instance::new(&graphics.entry, &graphics.instance);
            let surface = surface_loader
                .create_win32_surface(&create_info, None)
                .map_err(Win32Error::VulkanCreateSurfaceError)?;
            graphics
                .update_surface(surface, width, height)
                .map_err(Win32Error::VulkanUpdateSurfaceError)?;

            info!("WIN32 Window with Vulkan graphics created successfully");
            Ok(Win32Window {
                hwnd,
                hinstance,
                graphics,
            })
        }
    }

    fn event_loop(&self) -> Result<(), Win32Error> {
        let mut msg = MSG::default();

        unsafe {
            while GetMessageA(&mut msg, None, 0, 0).as_bool() {
                DispatchMessageA(&msg);
            }
        }

        Ok(())
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
                PostQuitMessage(0);
                LRESULT(0)
            }

            _ => DefWindowProcA(window, message, wparam, lparam),
        }
    }
}
