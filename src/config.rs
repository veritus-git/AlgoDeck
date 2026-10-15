use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_workspace_dir")]
    pub workspace_dir: String,

    #[serde(default = "default_vscode_mode")]
    pub vscode_mode: String,

    #[serde(default = "default_notifications")]
    pub notifications: bool,

    #[serde(default = "default_auto_detect_downloads")]
    pub auto_detect_downloads: bool,

    #[serde(default = "default_cpp_standard")]
    pub cpp_standard: String,

    #[serde(default)]
    pub sanitize_enabled: bool,

    #[serde(default = "default_open_pdf_in_vscode")]
    pub open_pdf_in_vscode: bool,

    #[serde(default = "default_compiler")]
    pub cxx_compiler: String,

    #[serde(default = "default_release_flags")]
    pub cxx_release_flags: String,

    #[serde(default = "default_debug_flags")]
    pub cxx_debug_flags: String,

    #[serde(default = "default_time_limit")]
    pub default_time_limit: f64,

    #[serde(default = "default_memory_limit")]
    pub default_memory_limit: u64,

    #[serde(default = "default_streamdeck_brightness")]
    pub streamdeck_brightness: u8,

    #[serde(default = "default_pdf_position")]
    pub pdf_position: String,

    #[serde(default = "default_editor_split_ratio")]
    pub editor_split_ratio: u8,

    #[serde(default = "default_autostart_on_boot")]
    pub autostart_on_boot: bool,
}

fn default_autostart_on_boot() -> bool {
    true
}

fn default_pdf_position() -> String {
    "right".to_string()
}

fn default_editor_split_ratio() -> u8 {
    60
}

fn default_streamdeck_brightness() -> u8 {
    85
}

fn default_workspace_dir() -> String {
    if let Some(home) = dirs::home_dir() {
        home.join("AlgoDeckWorkspace").to_string_lossy().to_string()
    } else {
        "./AlgoDeckWorkspace".to_string()
    }
}

fn default_vscode_mode() -> String {
    "auto".to_string()
}

fn default_notifications() -> bool {
    true
}

fn default_auto_detect_downloads() -> bool {
    true
}

fn default_cpp_standard() -> String {
    "c++20".to_string()
}

fn default_open_pdf_in_vscode() -> bool {
    true
}

fn default_compiler() -> String {
    "g++".to_string()
}

fn default_release_flags() -> String {
    "-O3 -Wall -Wextra".to_string()
}

fn default_debug_flags() -> String {
    "-O0 -g -fsanitize=address,undefined -Wall -Wextra".to_string()
}

fn default_time_limit() -> f64 {
    1.0
}

fn default_memory_limit() -> u64 {
    128
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            workspace_dir: default_workspace_dir(),
            vscode_mode: default_vscode_mode(),
            notifications: default_notifications(),
            auto_detect_downloads: default_auto_detect_downloads(),
            cpp_standard: default_cpp_standard(),
            sanitize_enabled: false,
            open_pdf_in_vscode: default_open_pdf_in_vscode(),
            cxx_compiler: default_compiler(),
            cxx_release_flags: default_release_flags(),
            cxx_debug_flags: default_debug_flags(),
            default_time_limit: default_time_limit(),
            default_memory_limit: default_memory_limit(),
            streamdeck_brightness: default_streamdeck_brightness(),
            pdf_position: default_pdf_position(),
            editor_split_ratio: default_editor_split_ratio(),
            autostart_on_boot: default_autostart_on_boot(),
        }
    }
}

impl AppSettings {
    pub fn config_path() -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            home.join(".config").join("algodeck").join("settings.json")
        } else {
            PathBuf::from("./settings.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                    return settings;
                }
            }
        }
        let default_settings = Self::default();
        let _ = default_settings.save();
        default_settings
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn get_workspace_path(&self) -> PathBuf {
        PathBuf::from(&self.workspace_dir)
    }

    pub fn staging_dir() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                return PathBuf::from(local).join("AlgoDeck").join("staging");
            }
        }
        if let Some(home) = dirs::home_dir() {
            home.join(".local").join("share").join("algodeck").join("staging")
        } else {
            PathBuf::from("/tmp/algodeck_staging")
        }
    }
}
