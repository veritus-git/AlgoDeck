pub mod app;
pub mod theme;
pub mod tray;

use app::AlgoDeckGuiApp;
use eframe::egui::ViewportBuilder;
use std::sync::Arc;

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

    let icon_data = load_icon();

    let mut viewport = ViewportBuilder::default()
        .with_title("AlgoDeck ⚡")
        .with_inner_size([520.0, 640.0])
        .with_min_inner_size([480.0, 560.0])
        .with_active(true);

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let native_options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "AlgoDeck",
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
