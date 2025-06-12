#[cfg(windows)]
pub mod win32;
#[cfg(windows)]
#[macro_export]
macro_rules! create_app {
    ($title:expr, $width:expr, $height:expr) => {{
        use yage2::platforms::win32::Win32Application;
        Win32Application::new($title, $width, $height)
    }};
}

#[cfg(target_os = "linux")]
pub mod x11;
#[cfg(target_os = "linux")]
#[macro_export]
macro_rules! create_app {
    ($application_config:expr) => {{
        use yage2::platforms::x11::X11Application;
        X11Application::new($application_config)
    }};
}
