use crate::window::Window;
use windows::Win32::Foundation::{
    GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, WIN32_ERROR, WPARAM,
};
use windows::Win32::Graphics::Gdi::UpdateWindow;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcA, DestroyWindow,
    DispatchMessageA, GetMessageA, MSG, PostQuitMessage, RegisterClassW, SW_SHOW, SetWindowTextW,
    ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WM_DESTROY, WM_PAINT, WNDCLASS_STYLES, WNDCLASSW,
    WNDPROC, WS_OVERLAPPEDWINDOW,
};
use windows::core::{HSTRING, PCWSTR};

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
}

#[derive(Debug)]
pub struct Win32Window {
    hwnd: HWND,
    hinstance: HINSTANCE,
}

fn get_last_error() -> WIN32_ERROR {
    unsafe { GetLastError() }
}

fn get_instance() -> Result<HINSTANCE, Win32Error> {
    match unsafe { GetModuleHandleW(None) } {
        Ok(handle) => Ok(HINSTANCE::from(handle)),
        Err(_) => Err(Win32Error::GetInstanceError(get_last_error())),
    }
}

fn register_class(
    class_name: &str,
    handle: HINSTANCE,
    flags: WNDCLASS_STYLES,
    wndproc: WNDPROC,
) -> Result<u16, Win32Error> {
    let class_name = HSTRING::from(class_name);

    if handle.is_invalid() {
        return Err(Win32Error::InvalidHINSTANCE(handle));
    }

    if class_name.is_empty() {
        return Err(Win32Error::InvalidClassName(class_name.to_string()));
    }

    match unsafe {
        let window_class = WNDCLASSW {
            style: flags,
            hInstance: handle,
            lpszClassName: PCWSTR(class_name.as_ptr()),
            lpfnWndProc: wndproc,

            ..Default::default()
        };
        RegisterClassW(&window_class)
    } {
        0 => Err(Win32Error::RegisterClassError(get_last_error())),
        atom => Ok(atom),
    }
}

fn create_window(
    class_name: &str,
    handle: HINSTANCE,
    title: &str,
    flags: WINDOW_STYLE,
) -> Result<HWND, Win32Error> {
    if handle.is_invalid() {
        return Err(Win32Error::InvalidHINSTANCE(handle));
    }

    if class_name.is_empty() {
        return Err(Win32Error::InvalidClassName(class_name.to_string()));
    }

    let class_name = HSTRING::from(class_name);
    let title = HSTRING::from(title);

    match unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(title.as_ptr()),
            flags,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            handle.into(),
            None,
        )
    } {
        Ok(hwnd) => Ok(hwnd),
        Err(_) => Err(Win32Error::CreateWindowError(get_last_error())),
    }
}

fn set_title(hwnd: HWND, title: &str) -> Result<(), Win32Error> {
    if hwnd.is_invalid() {
        return Err(Win32Error::InvalidHWND(hwnd));
    }

    let title = HSTRING::from(title);
    match unsafe { SetWindowTextW(hwnd, PCWSTR(title.as_ptr())) } {
        Ok(_) => Ok(()),
        Err(_) => Err(Win32Error::SetWindowTextError(get_last_error())),
    }
}

fn destroy_window(hwnd: HWND) -> Result<(), Win32Error> {
    if hwnd.is_invalid() {
        return Err(Win32Error::InvalidHWND(hwnd));
    }

    match unsafe { DestroyWindow(hwnd) } {
        Ok(_) => Ok(()),
        Err(_) => Err(Win32Error::CreateWindowError(get_last_error())),
    }
}

fn show_window(hwnd: HWND) -> Result<(), Win32Error> {
    if hwnd.is_invalid() {
        return Err(Win32Error::InvalidHWND(hwnd));
    }

    /* The function has no meaningful return value, so we ignore it. */
    let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };
    Ok(())
}

fn update_window(hwnd: HWND) -> Result<(), Win32Error> {
    if hwnd.is_invalid() {
        return Err(Win32Error::InvalidHWND(hwnd));
    }

    match unsafe { UpdateWindow(hwnd) }.as_bool() {
        true => Ok(()),
        false => Err(Win32Error::UpdateWindowError(get_last_error())),
    }
}

impl Drop for Win32Window {
    fn drop(&mut self) {
        let _ = destroy_window(self.hwnd);
    }
}

impl Window for Win32Window {
    type Error = Win32Error;

    fn new() -> Result<Self, Win32Error> {
        let hinstance = get_instance()?;

        const CLASS_NAME: &str = "Yage2 Window Class";
        register_class(
            CLASS_NAME,
            hinstance,
            CS_HREDRAW | CS_VREDRAW,
            Some(default_proc),
        )?;

        let hwnd = create_window(CLASS_NAME, hinstance, "Yage2 Window", WS_OVERLAPPEDWINDOW)?;

        Ok(Win32Window { hwnd, hinstance })
    }

    fn set_title(&self, title: &str) -> Result<(), Win32Error> {
        set_title(self.hwnd, title)
    }

    fn show(&self) -> Result<(), Win32Error> {
        show_window(self.hwnd)?;
        update_window(self.hwnd)?;
        Ok(())
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
                println!("WM_DESTROY");
                PostQuitMessage(0);
                LRESULT(0)
            }

            _ => {
                println!("Message: {}", message);
                DefWindowProcA(window, message, wparam, lparam)
            }
        }
    }
}
