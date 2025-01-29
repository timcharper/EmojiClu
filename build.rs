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

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let res_output = target_dir.join("app.res");

        println!("cargo:rerun-if-changed=app.rc");
        let status = std::process::Command::new("x86_64-w64-mingw32-windres")
            .args(&["app.rc", "-O", "coff", "-o"])
            .arg(&res_output)
            .status()
            .expect("Failed to run windres");

        if !status.success() {
            panic!("windres failed with exit code: {:?}", status.code());
        }

        // Tell Cargo to link the generated app.res
        println!("cargo:rustc-link-arg={}", res_output.display());
    }

    // Tell cargo to rerun if resources change
    println!("cargo:rerun-if-changed=resources");
}
