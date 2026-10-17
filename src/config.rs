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

    #[serde(default = "default_macos_window_mode")]
    pub macos_window_mode: String,
}

fn default_macos_window_mode() -> String {
    "default".to_string()
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
            macos_window_mode: default_macos_window_mode(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialDownloads {
    pub installed_at: u64,
    #[serde(default)]
    pub ignored_files: Vec<String>,
}

impl InitialDownloads {
    pub fn snapshot_path() -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            home.join(".config").join("algodeck").join("initial_downloads.json")
        } else {
            PathBuf::from("./initial_downloads.json")
        }
    }

    pub fn load_or_init() -> Self {
        let path = Self::snapshot_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(data) = serde_json::from_str::<InitialDownloads>(&content) {
                    return data;
                }
            }
        }

        let mut ignored = Vec::new();
        let dl_dir = dirs::download_dir().or_else(|| dirs::home_dir().map(|h| h.join("Downloads")));
        if let Some(dl) = dl_dir {
            if let Ok(entries) = fs::read_dir(dl) {
                for e in entries.flatten() {
                    if let Ok(file_type) = e.file_type() {
                        if file_type.is_file() {
                            let name = e.file_name().to_string_lossy().to_string();
                            ignored.push(name);
                        }
                    }
                }
            }
        }

        let installed_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let snapshot = InitialDownloads {
            installed_at,
            ignored_files: ignored,
        };

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(&snapshot) {
            let _ = fs::write(path, content);
        }

        snapshot
    }

    pub fn is_ignored(&self, file_path: &std::path::Path) -> bool {
        let filename = file_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if filename.is_empty() {
            return false;
        }

        let lower = filename.to_lowercase();
        let in_ignored_list = self
            .ignored_files
            .iter()
            .any(|ign| ign.eq_ignore_ascii_case(&filename) || ign.to_lowercase() == lower);

        if in_ignored_list {
            return true;
        }

        // Sprawdź czas modyfikacji pliku jeśli plik znajduje się w katalogu Downloads
        if let Ok(metadata) = file_path.metadata() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) {
                    let mod_millis = dur.as_millis() as u64;
                    if mod_millis <= self.installed_at + 3000 {
                        if let Some(dl) = dirs::download_dir().or_else(|| dirs::home_dir().map(|h| h.join("Downloads"))) {
                            if let Ok(can_dl) = dl.canonicalize() {
                                if let Ok(can_file) = file_path.canonicalize() {
                                    if can_file.starts_with(&can_dl) {
                                        return true;
                                    }
                                }
                            }
                            if file_path.starts_with(&dl) {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }
}
