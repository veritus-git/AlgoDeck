use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_workspace_dir")]
    pub workspace_dir: String,

    #[serde(default = "default_vscode_mode")]
    pub vscode_mode: String,

    #[serde(default = "default_notifications")]
    pub notifications: bool,

    #[serde(default = "default_notify_test_results")]
    pub notify_test_results: bool,

    #[serde(default = "default_notify_daemon_status")]
    pub notify_daemon_status: bool,

    #[serde(default = "default_notify_new_task_download")]
    pub notify_new_task_download: bool,

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

fn default_notify_test_results() -> bool {
    true
}

fn default_notify_daemon_status() -> bool {
    true
}

fn default_notify_new_task_download() -> bool {
    false
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
            notify_test_results: default_notify_test_results(),
            notify_daemon_status: default_notify_daemon_status(),
            notify_new_task_download: default_notify_new_task_download(),
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

        // Sprawdź czas modyfikacji pliku:
        // Jeśli plik został pobrany / zmodyfikowany PO instalacji AlgoDeck (poza marginesem 2s instalacji),
        // to jest to ŚWIEŻY PLIK, który trafił do folderu PO instalacji!
        // Użytkownik wprost wymaga: "dopiero swieze pliki ktore trafią do folderu pobranych PO instalacji mają byc brane pod uwage"
        if let Ok(metadata) = file_path.metadata() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) {
                    let mod_millis = dur.as_millis() as u64;
                    if mod_millis > self.installed_at + 2000 {
                        return false;
                    }
                }
            }
        }

        let lower = filename.to_lowercase();
        let in_ignored_list = self
            .ignored_files
            .iter()
            .any(|ign| ign.eq_ignore_ascii_case(&filename) || ign.to_lowercase() == lower);

        in_ignored_list
    }
}

pub fn get_downloads_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        if let Ok(out) = std::process::Command::new("xdg-user-dir").arg("DOWNLOAD").output() {
            let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path_str.is_empty() {
                let p = PathBuf::from(path_str);
                if p.exists() {
                    return p;
                }
            }
        }
    }

    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .unwrap_or_else(|| PathBuf::from("./Downloads"))
}

pub fn register_known_download_dir(dir: &Path) {
    if !dir.exists() || !dir.is_dir() {
        return;
    }
    let known_file = if let Some(h) = dirs::home_dir() {
        h.join(".config").join("algodeck").join("known_download_dirs.json")
    } else {
        PathBuf::from("./known_download_dirs.json")
    };
    if let Some(parent) = known_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut list: Vec<String> = if known_file.exists() {
        fs::read_to_string(&known_file)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let dir_str = dir.to_string_lossy().to_string();
    if !list.contains(&dir_str) {
        list.push(dir_str);
        if let Ok(serialized) = serde_json::to_string_pretty(&list) {
            let _ = fs::write(known_file, serialized);
        }
    }
}

pub fn get_all_download_dirs() -> Vec<PathBuf> {
    let mut dirs_set = HashSet::new();
    let mut result = Vec::new();

    let mut add_candidate = |p: PathBuf| {
        if p.exists() && p.is_dir() {
            let key = p.canonicalize().unwrap_or_else(|_| p.clone());
            if dirs_set.insert(key) {
                result.push(p);
            }
        }
    };

    // 1. Domyślny systemowy folder pobierania
    let sys_dl = get_downloads_dir();
    add_candidate(sys_dl.clone());
    add_candidate(sys_dl.join("Downloads"));
    add_candidate(sys_dl.join("Pobrane"));

    // 2. Dirs::download_dir i katalog domowy
    if let Some(d) = dirs::download_dir() {
        add_candidate(d.clone());
        add_candidate(d.join("Downloads"));
        add_candidate(d.join("Pobrane"));
    }
    if let Some(h) = dirs::home_dir() {
        add_candidate(h.join("Downloads"));
        add_candidate(h.join("Pobrane"));
        add_candidate(h.join("Pobrane").join("Downloads"));
        add_candidate(h.join("Downloads").join("Pobrane"));
    }

    // 3. Dynamicznie zarejestrowane katalogi (zgłoszone z rozszerzenia przez API)
    let known_file = if let Some(h) = dirs::home_dir() {
        h.join(".config").join("algodeck").join("known_download_dirs.json")
    } else {
        PathBuf::from("./known_download_dirs.json")
    };
    if known_file.exists() {
        if let Ok(txt) = fs::read_to_string(&known_file) {
            if let Ok(list) = serde_json::from_str::<Vec<String>>(&txt) {
                for d in list {
                    add_candidate(PathBuf::from(d));
                }
            }
        }
    }

    // 4. Zaawansowane wykrywanie per-przeglądarka z plików konfiguracyjnych (Brave, Chrome, Edge, Vivaldi, Opera)
    if let Some(h) = dirs::home_dir() {
        #[cfg(target_os = "linux")]
        let browser_config_roots = [
            h.join(".config/BraveSoftware/Brave-Browser"),
            h.join(".config/google-chrome"),
            h.join(".config/google-chrome-stable"),
            h.join(".config/chromium"),
            h.join(".config/microsoft-edge"),
            h.join(".config/microsoft-edge-stable"),
            h.join(".config/vivaldi"),
            h.join(".config/opera"),
        ];

        #[cfg(target_os = "macos")]
        let browser_config_roots = [
            h.join("Library/Application Support/BraveSoftware/Brave-Browser"),
            h.join("Library/Application Support/Google/Chrome"),
            h.join("Library/Application Support/Chromium"),
            h.join("Library/Application Support/Microsoft Edge"),
            h.join("Library/Application Support/Vivaldi"),
        ];

        #[cfg(target_os = "windows")]
        let mut browser_config_roots = Vec::new();
        #[cfg(target_os = "windows")]
        {
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                let p = PathBuf::from(local);
                browser_config_roots.push(p.join("BraveSoftware/Brave-Browser/User Data"));
                browser_config_roots.push(p.join("Google/Chrome/User Data"));
                browser_config_roots.push(p.join("Microsoft/Edge/User Data"));
            }
        }

        for root in &browser_config_roots {
            if root.exists() {
                if let Ok(entries) = fs::read_dir(root) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir() {
                            let pref = p.join("Preferences");
                            if pref.exists() {
                                if let Ok(content) = fs::read_to_string(&pref) {
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                                        if let Some(dl) = json.get("download").and_then(|d| d.get("default_directory")).and_then(|s| s.as_str()) {
                                            if !dl.is_empty() {
                                                add_candidate(PathBuf::from(dl));
                                            }
                                        }
                                        if let Some(save) = json.get("savefile").and_then(|d| d.get("default_directory")).and_then(|s| s.as_str()) {
                                            if !save.is_empty() {
                                                add_candidate(PathBuf::from(save));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Profile Mozilla Firefox
        #[cfg(target_os = "linux")]
        let ff_root = h.join(".mozilla/firefox");
        #[cfg(target_os = "macos")]
        let ff_root = h.join("Library/Application Support/Firefox/Profiles");
        #[cfg(target_os = "windows")]
        let ff_root = if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("Mozilla/Firefox/Profiles")
        } else {
            PathBuf::new()
        };

        if ff_root.exists() {
            if let Ok(entries) = fs::read_dir(&ff_root) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        let prefs = p.join("prefs.js");
                        if prefs.exists() {
                            if let Ok(content) = fs::read_to_string(&prefs) {
                                for line in content.lines() {
                                    if line.contains("browser.download.dir") {
                                        if let Some(val) = line.split('"').nth(3) {
                                            if !val.is_empty() {
                                                add_candidate(PathBuf::from(val));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if result.is_empty() {
        result.push(sys_dl);
    }
    result
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfRegistryEntry {
    pub filename: String,
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub modified_ms: u64,
    pub registered_at: u64,
    #[serde(default)]
    pub is_initial: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadedPdfRegistry {
    pub version: u32,
    pub updated_at: u64,
    pub entries: HashMap<String, PdfRegistryEntry>,
}

impl DownloadedPdfRegistry {
    pub fn registry_path() -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            home.join(".config").join("algodeck").join("downloaded_pdf_registry.json")
        } else {
            PathBuf::from("./downloaded_pdf_registry.json")
        }
    }

    pub fn compute_sha256(path: &Path) -> Result<String, std::io::Error> {
        let bytes = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    pub fn scan_and_register_initial_files(&mut self) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        for dl_dir in get_all_download_dirs() {
            if dl_dir.exists() {
                if let Ok(entries) = fs::read_dir(&dl_dir) {
                    for e in entries.flatten() {
                        let p = e.path();
                        if p.is_file() {
                            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                                if ext.eq_ignore_ascii_case("pdf") {
                                    if let Ok(hash) = Self::compute_sha256(&p) {
                                        let filename = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                                        let metadata = p.metadata().ok();
                                        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                                        let modified_ms = metadata
                                            .and_then(|m| m.modified().ok())
                                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                            .map(|d| d.as_millis() as u64)
                                            .unwrap_or(0);

                                        let key = filename.to_lowercase();
                                        if !self.entries.contains_key(&key) {
                                            let entry = PdfRegistryEntry {
                                                filename: filename.clone(),
                                                path: p.to_string_lossy().to_string(),
                                                sha256: hash,
                                                size,
                                                modified_ms,
                                                registered_at: now_ms,
                                                is_initial: true,
                                            };
                                            self.entries.insert(key, entry);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        self.updated_at = now_ms;
    }

    pub fn load_or_init() -> Self {
        let path = Self::registry_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(reg) = serde_json::from_str::<DownloadedPdfRegistry>(&content) {
                    return reg;
                }
            }
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let mut reg = DownloadedPdfRegistry {
            version: 1,
            updated_at: now_ms,
            entries: HashMap::new(),
        };

        reg.scan_and_register_initial_files();
        let _ = reg.save();
        reg
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::registry_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn check_and_register_if_new(&mut self, file_path: &Path) -> bool {
        if !file_path.exists() {
            return false;
        }

        let is_pdf = file_path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false);
        if !is_pdf {
            return false;
        }

        let hash = match Self::compute_sha256(file_path) {
            Ok(h) => h,
            Err(_) => return false,
        };

        let metadata = file_path.metadata().ok();
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified_ms = metadata
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let filename = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let key = filename.to_lowercase();

        let initial = InitialDownloads::load_or_init();
        let is_after_install = modified_ms > initial.installed_at + 2000;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        if let Some(existing) = self.entries.get(&key) {
            // Jeśli plik istniał przed instalacją i nie został pobrany/zmodyfikowany po instalacji:
            if existing.is_initial && !is_after_install {
                if existing.sha256 == hash && existing.size == size {
                    return false;
                }
            }
            // Zabezpieczenie przed podwójnym zarejestrowaniem tego samego zdarzenia (burst do 3 sekund)
            if !existing.is_initial && existing.sha256 == hash && existing.size == size && now_ms.saturating_sub(existing.registered_at) < 3000 {
                return false;
            }
        }

        let entry = PdfRegistryEntry {
            filename: filename.clone(),
            path: file_path.to_string_lossy().to_string(),
            sha256: hash,
            size,
            modified_ms,
            registered_at: now_ms,
            is_initial: false,
        };

        self.entries.insert(key, entry);
        self.updated_at = now_ms;
        let _ = self.save();
        true
    }

    #[allow(dead_code)]
    pub fn is_already_known(&self, file_path: &Path) -> bool {
        if !file_path.exists() {
            return false;
        }
        let filename = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let key = filename.to_lowercase();

        if let Some(existing) = self.entries.get(&key) {
            if let Ok(metadata) = file_path.metadata() {
                if metadata.len() == existing.size {
                    if let Ok(hash) = Self::compute_sha256(file_path) {
                        if hash == existing.sha256 {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}
