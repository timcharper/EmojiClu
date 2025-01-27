use crate::model::Difficulty;
use glib;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub difficulty: Difficulty,
    pub clue_tooltips_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            difficulty: Difficulty::default(),
            clue_tooltips_enabled: true,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = Self::settings_path();
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str(&contents) {
                return settings;
            }
        }
        let default = Settings::default();
        // Try to save default settings, but don't panic if it fails
        let _ = default.save();
        default
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::settings_path();
        // Ensure the directory exists
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)
    }

    fn settings_path() -> PathBuf {
        let data_dir = glib::user_data_dir();
        let mut path = data_dir.join("gwatson");
        path.push("settings.json");
        path
    }
}
