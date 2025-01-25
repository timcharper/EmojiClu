use std::fs;
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    // Compile the resources
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.xml",
        "compiled.gresource",
    );

    // Copy to a consistent location
    let target_dir = Path::new("target/resources");
    fs::create_dir_all(target_dir).unwrap();
    fs::copy(
        Path::new(&out_dir).join("compiled.gresource"),
        target_dir.join("compiled.gresource"),
    )
    .unwrap();

    // Tell cargo to rerun if resources change
    println!("cargo:rerun-if-changed=resources");
}
