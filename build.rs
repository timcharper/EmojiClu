use glib_build_tools::compile_resources;
use image::imageops::FilterType;
use image::ImageReader;
use std::fs;
use std::path::Path;

fn should_update(source: &Path, target: &Path) -> bool {
    if !target.exists() {
        return true;
    }

    let source_modified = fs::metadata(source)
        .and_then(|m| m.modified())
        .expect("Failed to get source modification time");
    let target_modified = fs::metadata(target)
        .and_then(|m| m.modified())
        .expect("Failed to get target modification time");

    source_modified > target_modified
}

fn generate_app_icons(source: &Path, target_dir: &Path) {
    // Icon sizes commonly used by Linux desktop environments
    let sizes = [16, 24, 32, 48, 64, 128, 256, 512];
    let source_img = ImageReader::open(source).unwrap().decode().unwrap();

    for size in sizes {
        let size_dir = target_dir.join(format!("{size}x{size}")).join("apps");
        fs::create_dir_all(&size_dir).unwrap();

        let target_icon = size_dir.join("org.timcharper.EmojiClu.png");
        if should_update(source, &target_icon) {
            let resized = image::imageops::resize(&source_img, size, size, FilterType::Lanczos3);
            resized.save(target_icon).unwrap();
        }
    }
}

fn main() {
    let version = env!("CARGO_PKG_VERSION").to_string();

    println!("cargo:rustc-env=APP_VERSION={}", version);

    // Compile the resources
    compile_resources(
        &["resources"],
        "resources/resources.xml",
        "compiled.gresource",
    );

    // Set up target directory for resources based on profile
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let profile = std::env::var("PROFILE").unwrap();
    let target_dir = Path::new("target").join(&profile);
    let resources_dir = target_dir.join("resources");
    fs::create_dir_all(&resources_dir).unwrap();

    // Copy the compiled resource
    fs::copy(
        Path::new(&out_dir).join("compiled.gresource"),
        target_dir.join("compiled.gresource"),
    )
    .unwrap();

    // Set up icon theme directory
    let icon_theme_dir = resources_dir.join("icons/hicolor");
    fs::create_dir_all(&icon_theme_dir).unwrap();

    // Generate icons from the source icon
    generate_app_icons(Path::new("resources/emojiclu-icon.png"), &icon_theme_dir);

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let res_output = resources_dir.join("app.res");

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
