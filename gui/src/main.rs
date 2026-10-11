mod api;
mod app;
mod settings;
mod theme;
mod tray;

use app::{ActiveTab, AlgoDeckApp};
use eframe::egui::{pos2, vec2, Pos2, ViewportBuilder};
use std::env;

fn get_primary_screen_pos(win_w: f32, win_h: f32) -> Option<Pos2> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(out) = std::process::Command::new("xrandr").arg("--current").output() {
            if let Ok(text) = String::from_utf8(out.stdout) {
                let mut prim_line = None;
                for line in text.lines() {
                    if line.contains(" connected primary") {
                        prim_line = Some(line);
                        break;
                    }
                }
                if prim_line.is_none() {
                    for line in text.lines() {
                        if line.contains(" connected") {
                            prim_line = Some(line);
                            break;
                        }
                    }
                }
                if let Some(line) = prim_line {
                    for token in line.split_whitespace() {
                        if let Some((wh, xy)) = token.split_once('+') {
                            if let Some((w_s, h_s)) = wh.split_once('x') {
                                if let Some((x_s, y_s)) = xy.split_once('+') {
                                    if let (Ok(w), Ok(h), Ok(x), Ok(y)) = (
                                        w_s.parse::<f32>(),
                                        h_s.parse::<f32>(),
                                        x_s.parse::<f32>(),
                                        y_s.parse::<f32>(),
                                    ) {
                                        let center_x = x + (w - win_w) / 2.0;
                                        let center_y = y + (h - win_h) / 2.0;
                                        return Some(pos2(center_x, center_y));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().collect();

    // 1. Tryb dedykowanego daemona zasobnika systemowego (bez okna GUI)
    if args.iter().any(|a| a == "--tray") {
        if !tray::is_tray_running() {
            tray::spawn_tray();
        }
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }

    let is_launcher_launch = args.iter().any(|a| a == "--launcher-status" || a == "--app-launch");

    // 2. Włącz ikonę zasobnika systemowego w tle TYLKO jeśli daemon nie jest jeszcze aktywny
    if !tray::is_tray_running() {
        tray::spawn_tray();
    }

    let mut initial_tab = ActiveTab::NewTask;
    let mut initial_pdf = None;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--tab" && i + 1 < args.len() {
            match args[i + 1].as_str() {
                "tasks" => initial_tab = ActiveTab::TasksList,
                "settings" => initial_tab = ActiveTab::Settings,
                _ => initial_tab = ActiveTab::NewTask,
            }
            i += 2;
        } else if args[i].starts_with("--tab=") {
            let tab_val = args[i].trim_start_matches("--tab=");
            match tab_val {
                "tasks" => initial_tab = ActiveTab::TasksList,
                "settings" => initial_tab = ActiveTab::Settings,
                _ => initial_tab = ActiveTab::NewTask,
            }
            i += 1;
        } else if args[i] == "--pdf" && i + 1 < args.len() {
            initial_pdf = Some(args[i + 1].clone());
            i += 2;
        } else if args[i].starts_with("--pdf=") {
            initial_pdf = Some(args[i].trim_start_matches("--pdf=").to_string());
            i += 1;
        } else {
            i += 1;
        }
    }

    let mut app = AlgoDeckApp::new(initial_pdf);
    app.current_tab = initial_tab;
    app.is_launcher_status = is_launcher_launch;

    // Nowoczesne, węższe i wyższe proporcje: 520x680 dla pełnego panelu lub 480x240 dla launchera
    let (win_size, min_size) = if is_launcher_launch {
        (vec2(480.0, 240.0), vec2(420.0, 220.0))
    } else {
        (vec2(520.0, 680.0), vec2(460.0, 580.0))
    };

    let mut viewport = ViewportBuilder::default()
        .with_title("AlgoDeck ⚡ Centrum Kontroli")
        .with_inner_size(win_size)
        .with_min_inner_size(min_size)
        .with_resizable(true)
        .with_decorations(false)
        .with_transparent(false);

    if let Some(pos) = get_primary_screen_pos(win_size.x, win_size.y) {
        viewport = viewport.with_position(pos);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "AlgoDeck ⚡ Centrum Kontroli",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
