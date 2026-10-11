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
        "applications-development".to_string()
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
