use std::path::Path;
use std::process::Command;

#[cfg(target_os = "linux")]
use ksni::{blocking::TrayMethods, menu::*, MenuItem, Tray};

pub fn ensure_system_icon() {
    let icon_bytes = include_bytes!("../../assets/icon128.png");
    if let Some(data_dir) = dirs::data_dir() {
        let icon_paths = [
            data_dir.join("icons").join("algodeck.png"),
            data_dir.join("icons").join("hicolor").join("128x128").join("apps").join("algodeck.png"),
        ];
        for path in icon_paths {
            if !path.exists() {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&path, icon_bytes);
            }
        }
    }
}

pub fn get_algodeck_tray_icons() -> Vec<ksni::Icon> {
    let img_bytes = include_bytes!("../../assets/icon128.png");
    if let Ok(img) = image::load_from_memory_with_format(img_bytes, image::ImageFormat::Png) {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let mut data = rgba.into_raw();
        // ARGB32 format dla StatusNotifierItem DBus
        for pixel in data.chunks_exact_mut(4) {
            pixel.rotate_right(1);
        }
        vec![ksni::Icon {
            width: w as i32,
            height: h as i32,
            data,
        }]
    } else {
        vec![]
    }
}

pub struct AlgoDeckTray;

#[cfg(target_os = "linux")]
impl Tray for AlgoDeckTray {
    const MENU_ON_ACTIVATE: bool = false;

    fn id(&self) -> String {
        "algodeck".to_string()
    }

    fn title(&self) -> String {
        "AlgoDeck ⚡".to_string()
    }

    fn icon_name(&self) -> String {
        ensure_system_icon();
        "algodeck".to_string()
    }

    fn icon_theme_path(&self) -> String {
        ensure_system_icon();
        if let Some(data_dir) = dirs::data_dir() {
            let path = data_dir.join("icons");
            if path.exists() {
                return path.to_string_lossy().to_string();
            }
        }
        "".to_string()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        get_algodeck_tray_icons()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        open_control_panel();
    }

    fn secondary_activate(&mut self, _x: i32, _y: i32) {
        open_control_panel();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "⚡ AlgoDeck (100% Rust)".into(),
                enabled: false,
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: "🖥 Otwórz AlgoDeck".into(),
                activate: Box::new(|_| {
                    open_control_panel();
                }),
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: "🛑 Wyłącz AlgoDeck".into(),
                activate: Box::new(|_| {
                    stop_algodeck();
                    std::process::exit(0);
                }),
                ..Default::default()
            }.into(),
        ]
    }
}

pub fn is_tray_running() -> bool {
    let my_pid = std::process::id();
    let pid_file = std::env::temp_dir().join("algodeck_tray.pid");
    if pid_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                if pid != my_pid {
                    #[cfg(target_os = "linux")]
                    {
                        if Path::new(&format!("/proc/{}", pid)).exists() {
                            if let Ok(cmdline) = std::fs::read_to_string(format!("/proc/{}/cmdline", pid)) {
                                if cmdline.contains("algodeck") {
                                    return true;
                                }
                            }
                        }
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn acquire_tray_lock() -> bool {
    if is_tray_running() {
        return false;
    }
    let pid_file = std::env::temp_dir().join("algodeck_tray.pid");
    let _ = std::fs::write(pid_file, format!("{}", std::process::id()));
    true
}

pub fn open_control_panel() {
    let _ = Command::new("algodeck").spawn();
}

pub fn stop_algodeck() {
    let pid_file = std::env::temp_dir().join("algodeck_tray.pid");
    let _ = std::fs::remove_file(pid_file);
    let _ = Command::new("algodeck").arg("stop").spawn();
}

#[cfg(target_os = "linux")]
pub fn spawn_tray() {
    if acquire_tray_lock() {
        ensure_system_icon();
        let _ = AlgoDeckTray.spawn();
    }
}

#[cfg(not(target_os = "linux"))]
pub fn spawn_tray() {}
