mod game;
mod model;
mod ui;

use gio::glib::Bytes;
use gtk::prelude::*;
use gtk::Application;

const APP_ID: &str = "org.gwatson.LogicPuzzle";

// At the top of the file, include the compiled resources
const RESOURCES: &[u8] = include_bytes!("../target/resources/compiled.gresource");

fn init_logging() {
    env_logger::init();
}

fn main() {
    #[cfg(target_os = "windows")]
    {
        use gtk::ffi::gtk_icon_theme_get_search_path;
        use gtk::gdk::Display;
        use std::env;
        use std::path::Path;
        env::set_var("GTK_THEME", "Adwaita");
        gtk::init().unwrap();

        // // let icons = gtk::IconTheme::default();
        let exe_path = env::current_exe().expect("Failed to get current executable path");
        // println!("exe_path: {:?}", exe_path);

        let icons = gtk::IconTheme::for_display(
            &Display::default().expect("Could not connect to a display."),
        );

        icons.add_search_path(
            Path::new(&exe_path.parent().unwrap().parent().unwrap()).join("share/icons"),
        );
    }

    init_logging();

    // Register resources before creating the application
    gio::resources_register(&gio::Resource::from_data(&Bytes::from_static(RESOURCES)).unwrap());

    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal
    app.connect_activate(ui::window::build_ui);

    // Run the application
    app.run();
}
