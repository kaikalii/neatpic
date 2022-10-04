use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub window_width: i32,
    pub window_height: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            window_width: 1280,
            window_height: 720,
        }
    }
}

fn app_dir() -> PathBuf {
    let path = dirs::data_local_dir().unwrap_or_default().join("neatpic");
    if let Err(e) = fs::create_dir_all(&path) {
        eprintln!("{e}");
    }
    path
}

impl Settings {
    pub fn path() -> PathBuf {
        app_dir().join("settings.yaml")
    }
    pub fn load() -> Self {
        fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| serde_yaml::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) {
        if let Err(e) = fs::write(Self::path(), serde_yaml::to_string(self).unwrap()) {
            eprintln!("{e}");
        }
    }
}
