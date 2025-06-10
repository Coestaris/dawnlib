#[cfg(windows)]
pub(crate) mod win32;
#[cfg(windows)]
#[macro_export] macro_rules! create_window {
    ($title:expr, $width:expr, $height:expr) => {
        crate::platforms::win32::Win32Window::new($title, $width, $height)
    };
}

#[cfg(target_os = "linux")]
pub(crate) mod x11;
#[cfg(target_os = "linux")]
#[macro_export] macro_rules! create_window {
    ($title:expr, $width:expr, $height:expr) => {
        crate::platforms::x11::X11Window::new($title, $width, $height)
    };
}
