mod api;
mod app;
mod settings;
mod theme;

use app::{ActiveTab, AlgoDeckApp};
use eframe::egui::{vec2, ViewportBuilder};
use std::env;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().collect();
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

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("AlgoDeck ⚡ Centrum Kontroli")
            .with_inner_size(vec2(420.0, 500.0))
            .with_min_inner_size(vec2(420.0, 500.0))
            .with_max_inner_size(vec2(420.0, 500.0))
            .with_resizable(false)
            .with_decorations(false) // Własny customowy pasek tytułowy BadWords-style!
            .with_transparent(false),
        ..Default::default()
    };

    eframe::run_native(
        "AlgoDeck ⚡ Centrum Kontroli",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
