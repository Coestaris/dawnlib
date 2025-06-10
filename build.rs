use std::env;
use std::fs;
use std::path::Path;

fn main() {
    /* If running on Linux, dynamically link to the X11 libraries */
    if cfg!(target_os = "linux") {
        // Link to the X11 library
        println!("cargo:rustc-link-lib=dylib=X11");
    }
}
