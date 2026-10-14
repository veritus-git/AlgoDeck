pub mod app;
pub mod theme;
pub mod tray;

use app::AlgoDeckGuiApp;
use eframe::egui::{pos2, vec2, Pos2, ViewportBuilder};
use std::sync::Arc;

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

pub fn run_gui(initial_pdf: Option<String>, tray_only: bool) -> Result<(), eframe::Error> {
    tray::ensure_system_icon();

    if tray_only {
        #[cfg(target_os = "linux")]
        {
            tray::spawn_tray();
            std::thread::park();
            return Ok(());
        }
    } else {
        #[cfg(target_os = "linux")]
        {
            tray::spawn_tray();
        }
    }

    let win_size = vec2(340.0, 580.0);
    let min_size = vec2(300.0, 480.0);

    let mut viewport = ViewportBuilder::default()
        .with_title("AlgoDeck ⚡ Centrum Kontroli")
        .with_inner_size(win_size)
        .with_min_inner_size(min_size)
        .with_resizable(true)
        .with_decorations(false)
        .with_transparent(false)
        .with_active(true);

    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }

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
        Box::new(|cc| {
            theme::configure_theme(&cc.egui_ctx);
            Ok(Box::new(AlgoDeckGuiApp::new(initial_pdf)))
        }),
    )
}

fn load_icon() -> Option<egui::IconData> {
    let img_bytes = include_bytes!("../../assets/icon128.png");
    if let Ok(img) = image::load_from_memory_with_format(img_bytes, image::ImageFormat::Png) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        })
    } else {
        None
    }
}
