use vergen::EmitBuilder;

fn main() {
    /* If running on Linux, dynamically link to the X11 libraries */
    if cfg!(target_os = "linux") {
        // Link to the X11 library
        println!("cargo:rustc-link-lib=dylib=X11");
    }

    EmitBuilder::builder()
        .all_build()
        .all_cargo()
        .all_git()
        .all_rustc()
        .all_sysinfo()
        .emit()
        .unwrap();
}
