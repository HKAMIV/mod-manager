use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub id: String,
    pub name: String,
    pub mod_path: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub active_game: String,
    pub games: Vec<GameConfig>,
    pub nsfw_filter: bool,
    pub auto_reload: bool,
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            active_game: "wuthering-waves".to_string(),
            games: vec![
                GameConfig {
                    id: "wuthering-waves".to_string(),
                    name: "Wuthering Waves".to_string(),
                    mod_path: None,
                    enabled: true,
                },
                GameConfig {
                    id: "genshin-impact".to_string(),
                    name: "Genshin Impact".to_string(),
                    mod_path: None,
                    enabled: true,
                },
                GameConfig {
                    id: "zenless-zone-zero".to_string(),
                    name: "Zenless Zone Zero".to_string(),
                    mod_path: None,
                    enabled: true,
                },
                GameConfig {
                    id: "honkai-star-rail".to_string(),
                    name: "Honkai Star Rail".to_string(),
                    mod_path: None,
                    enabled: true,
                },
                GameConfig {
                    id: "arknights-endfield".to_string(),
                    name: "Arknights Endfield".to_string(),
                    mod_path: None,
                    enabled: true,
                },
            ],
            nsfw_filter: true,
            auto_reload: true,
            theme: "dark".to_string(),
        }
    }
}

pub struct AppState {
    pub settings: Mutex<Settings>,
}

impl AppState {
    pub fn new() -> Self {
        let settings = Self::load_settings().unwrap_or_default();
        Self {
            settings: Mutex::new(settings),
        }
    }

    pub fn config_dir() -> PathBuf {
        let config = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"));
        config.join("mod-manager")
    }

    /// XDG data directory for larger, non-config artifacts — currently used
    /// for restore point file backups, which can be sizable and don't belong
    /// alongside small JSON config/preset files.
    pub fn data_dir() -> PathBuf {
        let data = dirs::data_dir().unwrap_or_else(|| PathBuf::from("~/.local/share"));
        data.join("mod-manager")
    }

    pub fn settings_path() -> PathBuf {
        Self::config_dir().join("settings.json")
    }

    fn load_settings() -> Option<Settings> {
        let path = Self::settings_path();
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn save_settings(settings: &Settings) -> Result<(), String> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = Self::settings_path();
        let data = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        fs::write(path, data).map_err(|e| e.to_string())?;
        Ok(())
    }
}
