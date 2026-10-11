use std::process::Command;

#[cfg(target_os = "linux")]
use ksni::{blocking::TrayMethods, menu::*, MenuItem, Tray};

pub struct AlgoDeckTray;

#[cfg(target_os = "linux")]
impl Tray for AlgoDeckTray {
    fn id(&self) -> String {
        "algodeck".to_string()
    }

    fn title(&self) -> String {
        "AlgoDeck ⚡".to_string()
    }

    fn icon_name(&self) -> String {
        "algodeck".to_string()
    }

    fn icon_theme_path(&self) -> String {
        if let Some(data_dir) = dirs::data_dir() {
            let path = data_dir.join("icons");
            if path.exists() {
                return path.to_string_lossy().to_string();
            }
        }
        "".to_string()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        // Precyzyjna, wektorowo wyliczana ikona 32x32: błękitny piorun na ciemnym tle (ARGB32)
        const SIZE: i32 = 32;
        let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);

        for y in 0..SIZE {
            for x in 0..SIZE {
                // Zaokrąglony kafel
                let dx = (x - 16).abs();
                let dy = (y - 16).abs();
                let in_card = dx <= 14 && dy <= 14;

                // Geometria geometrycznego pioruna ⚡
                // Górny trójkąt: (18, 5) -> (8, 17) -> (16, 17)
                // Dolny trójkąt: (14, 15) -> (22, 15) -> (12, 27)
                let is_bolt = (y >= 5 && y <= 16 && x >= (18 - (y - 5)) && x <= 22)
                    || (y >= 15 && y <= 27 && x >= 10 && x <= (14 + (27 - y) * 8 / 12));

                if in_card {
                    if is_bolt {
                        // Cyan #38bdf8
                        data.push(255); // Alpha
                        data.push(56);  // Red
                        data.push(189); // Green
                        data.push(248); // Blue
                    } else {
                        // Ciemne tło #0d1322
                        data.push(255);
                        data.push(13);
                        data.push(19);
                        data.push(34);
                    }
                } else {
                    // Przezroczyste tło
                    data.push(0);
                    data.push(0);
                    data.push(0);
                    data.push(0);
                }
            }
        }

        vec![ksni::Icon {
            width: SIZE,
            height: SIZE,
            data,
        }]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        open_control_panel();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "⚡ AlgoDeck (ONLINE)".into(),
                enabled: false,
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: "🖥 Centrum Kontroli".into(),
                activate: Box::new(|_| {
                    open_control_panel();
                }),
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: "🛑 Wyłącz AlgoDeck i Stream Deck".into(),
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
    if let Ok(output) = Command::new("pgrep").args(["-f", "algodeck-gui --tray"]).output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if let Ok(pid) = line.trim().parse::<u32>() {
                if pid != my_pid {
                    return true;
                }
            }
        }
    }
    false
}

pub fn open_control_panel() {
    let _ = Command::new("algodeck").arg("gui").spawn();
}

pub fn stop_algodeck() {
    let _ = Command::new("algodeck").arg("stop").spawn();
}

#[cfg(target_os = "linux")]
pub fn spawn_tray() {
    let _ = AlgoDeckTray.spawn();
}

#[cfg(not(target_os = "linux"))]
pub fn spawn_tray() {}
