use emojiclu::ui::build_ui;
use fluent_i18n::set_locale;
use gio::prelude::*;
use gio::ApplicationFlags;
use glib::{Bytes, ExitCode};
use gtk4::Application;

const APP_ID: &str = "io.github.timcharper.EmojiClu";

#[cfg(debug_assertions)]
const RESOURCES: &[u8] = include_bytes!("../target/debug/compiled.gresource");

#[cfg(not(debug_assertions))]
const RESOURCES: &[u8] = include_bytes!("../target/release/compiled.gresource");

fn main() -> ExitCode {
    // Initialize logger
    env_logger::init();

    // Set locale from environment variable if provided
    if let Ok(locale) = std::env::var("LOCALE") {
        if let Err(e) = set_locale(Some(&locale)) {
            eprintln!("Warning: Failed to set locale '{}': {}", locale, e);
        } else {
            println!("Locale set to: {}", locale);
        }
    }

    #[cfg(target_os = "windows")]
    {
        use gtk4::gdk::Display;
        use std::env;
        use std::path::Path;
        env::set_var("GTK_THEME", "Adwaita");
        gtk4::init().unwrap();

        // // let icons = gtk::IconTheme::default();
        let exe_path = env::current_exe().expect("Failed to get current executable path");
        // println!("exe_path: {:?}", exe_path);

        let icons = gtk4::IconTheme::for_display(
            &Display::default().expect("Could not connect to a display."),
        );

        icons.add_search_path(
            Path::new(&exe_path.parent().unwrap().parent().unwrap()).join("share/icons"),
        );
    }

    // Register resources before creating the application
    gio::resources_register(&gio::Resource::from_data(&Bytes::from_static(RESOURCES)).unwrap());

    // Create a new application
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(ApplicationFlags::empty())
        .build();

    // Connect to "activate" signal
    app.connect_activate(|app| build_ui(app));

    // Run the application
    app.run()
}
