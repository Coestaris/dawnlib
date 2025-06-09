extern crate core;

use crate::win32::Win32Window;
use crate::window::Window;

mod window;
mod win32;


fn main() {
    let window = Win32Window::new().unwrap_or_else(
        |error| { panic!("Cannot create a window {:?}", error) }
    );
        
    window.set_title("Hello, world!").unwrap();
    window.show().unwrap();
    window.event_loop().unwrap();
}
