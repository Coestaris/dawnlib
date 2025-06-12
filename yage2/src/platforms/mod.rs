#[cfg(windows)]
pub mod win32;
#[cfg(windows)]
#[macro_export] macro_rules! create_window {
    ($title:expr, $width:expr, $height:expr) => { {
            use yage2::platforms::win32::Win32Window;
            Win32Window::new($title, $width, $height)
        }
    };
}

#[cfg(target_os = "linux")]
pub mod x11;
#[cfg(target_os = "linux")]
#[macro_export] macro_rules! create_window {
    ($title:expr, $width:expr, $height:expr) => { {
            use yage2::platforms::x11::X11Window;
            X11Window::new($title, $width, $height)
        }
    };
}
