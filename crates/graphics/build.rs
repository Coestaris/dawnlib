extern crate gl_generator;
use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=X11");

    #[cfg(all(target_os = "linux", feature = "gl"))]
    println!("cargo:rustc-link-lib=GL");

    #[cfg(feature = "gl")]
    {
        let dest = env::var("OUT_DIR").unwrap();
        let mut file = File::create(&Path::new(&dest).join("gl_bindings.rs")).unwrap();
        Registry::new(
            Api::Gl,
            (4, 5),
            Profile::Compatibility,
            Fallbacks::All,
            ["GLX_ARB_create_context"],
        )
        .write_bindings(GlobalGenerator, &mut file)
        .unwrap();
    }
}
