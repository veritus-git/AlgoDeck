use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub workspace_dir: String,
    pub vscode_mode: String,
    pub notifications: bool,
    #[serde(default = "default_true")]
    pub auto_detect_downloads: bool,
    pub cpp_standard: String,
    pub sanitize_enabled: bool,
    pub open_pdf_in_vscode: bool,
    pub streamdeck_platform: String,
    pub streamdeck_serial: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let default_workspace = home.join("algodeck-workspace");
        Self {
            workspace_dir: default_workspace.to_string_lossy().to_string(),
            vscode_mode: "single_window".to_string(),
            notifications: true,
            auto_detect_downloads: true,
            cpp_standard: "c++20".to_string(),
            sanitize_enabled: false,
            open_pdf_in_vscode: false,
            streamdeck_platform: if cfg!(target_os = "macos") {
                "macos_elgato".to_string()
            } else if cfg!(target_os = "windows") {
                "windows_elgato".to_string()
            } else {
                "linux_streamcontroller".to_string()
            },
            streamdeck_serial: "A00SA6042JGA63".to_string(),
        }
    }
}

impl AppSettings {
    pub fn config_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("algodeck").join("settings.json")
        } else {
            PathBuf::from("settings.json")
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
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(())
    }
}
