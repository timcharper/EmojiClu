use crate::model::Difficulty;
use glib;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "default_version")]
    version: u32,

    #[serde(default)]
    pub difficulty: Difficulty,

    #[serde(default = "default_true")]
    pub clue_tooltips_enabled: bool,

    #[serde(default)]
    pub clue_spotlight_enabled: bool,

    #[serde(default)]
    pub touch_screen_controls: bool,
}

// Helper functions for default values
fn default_version() -> u32 {
    1
}
fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            difficulty: Difficulty::default(),
            clue_tooltips_enabled: true,
            clue_spotlight_enabled: false,
            touch_screen_controls: false,
            version: 1,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = Self::settings_path();
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(mut settings) = serde_json::from_str::<Settings>(&contents) {
                settings.migrate();
                return settings;
            }
        }
        let default = Settings::default();
        let _ = default.save();
        default
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::settings_path();
        // Ensure the directory exists
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let contents = serde_json::to_string(self)?;
        fs::write(path, contents)
    }

    fn settings_path() -> PathBuf {
        let data_dir = glib::user_data_dir();
        let mut path = data_dir.join("mindhunt");
        path.push("settings.json");
        path
    }

    fn migrate(&mut self) {
        match self.version {
            0 => {
                self.version = 1;
            }
            _ => (),
        }
    }

    pub fn is_debug_mode() -> bool {
        std::env::var("DEBUG").map(|v| v == "1").unwrap_or(false)
    }

    pub fn seed_from_env() -> Option<u64> {
        std::env::var("SEED")
            .map(|v| v.parse::<u64>().unwrap())
            .ok()
    }
}
