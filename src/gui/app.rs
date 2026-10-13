use egui::{pos2, vec2, Align, Align2, Color32, Layout, RichText, Rounding, Stroke, Ui};
use std::path::Path;
use std::process::Command;

use crate::config::AppSettings;
use crate::gui::theme::*;
use crate::pdf::PdfParser;
use crate::workspace::{ProblemSummary, WorkspaceManager};

#[derive(PartialEq, Clone, Copy)]
pub enum ActiveTab {
    Import,
    Manual,
    Tasks,
    Settings,
}

pub struct AlgoDeckGuiApp {
    pub current_tab: ActiveTab,
    pub settings: AppSettings,

    // Import state
    pub pdf_path: String,
    pub zip_path: String,
    pub task_code: String,
    pub task_title: String,
    pub time_limit: f64,
    pub memory_limit: u64,
    pub detected_tests_count: usize,
    pub is_submitting: bool,

    // Manual state
    pub manual_cin: String,
    pub manual_cout: String,

    // Tasks list
    pub problems: Vec<ProblemSummary>,
    pub active_problem_id: String,
    pub search_query: String,

    // Offline Hero state
    pub is_backend_online: bool,
    pub is_starting_daemon: bool,
    pub show_settings_offline: bool,

    // Status toast
    pub status_message: Option<(String, bool)>, // (msg, is_error)
}

impl AlgoDeckGuiApp {
    pub fn new(initial_pdf: Option<String>) -> Self {
        let settings = AppSettings::load();
        let ws_dir = settings.get_workspace_path();
        let problems = WorkspaceManager::list_problems(&ws_dir);

        let is_online = Self::check_backend_alive();
        let active_id = Self::fetch_active_problem_id();

        let mut app = Self {
            current_tab: ActiveTab::Import,
            settings,
            pdf_path: String::new(),
            zip_path: String::new(),
            task_code: String::new(),
            task_title: String::new(),
            time_limit: 1.0,
            memory_limit: 128,
            detected_tests_count: 0,
            is_submitting: false,
            manual_cin: String::new(),
            manual_cout: String::new(),
            problems,
            active_problem_id: active_id,
            search_query: String::new(),
            is_backend_online: is_online,
            is_starting_daemon: false,
            show_settings_offline: false,
            status_message: None,
        };

        if let Some(pdf) = initial_pdf {
            if Path::new(&pdf).exists() {
                app.set_pdf(pdf);
            }
        } else {
            // Check staging for pre-existing files
            app.check_staging_files();
        }

        app
    }

    fn check_backend_alive() -> bool {
        match ureq::get("http://127.0.0.1:8080/api/status")
            .timeout(std::time::Duration::from_millis(400))
            .call()
        {
            Ok(r) => r.status() == 200,
            Err(_) => false,
        }
    }

    fn fetch_active_problem_id() -> String {
        if let Ok(res) = ureq::get("http://127.0.0.1:8080/api/status")
            .timeout(std::time::Duration::from_millis(400))
            .call()
        {
            if let Ok(json) = res.into_json::<serde_json::Value>() {
                if let Some(act) = json.get("active_problem").and_then(|v| v.as_str()) {
                    return act.to_string();
                }
            }
        }
        String::new()
    }

    pub fn set_pdf(&mut self, path: String) {
        self.pdf_path = path.clone();
        if let Ok(text) = PdfParser::extract_text_from_file(Path::new(&path)) {
            let analysis = PdfParser::parse_heuristics(&text);
            self.task_code = analysis.problem_id;
            self.task_title = analysis.title;
            self.time_limit = analysis.time_limit_sec;
            self.memory_limit = analysis.memory_limit_mb;
            self.detected_tests_count = analysis.tests.len();
            self.set_status("Sparsowano metadane z pliku PDF!", false);
        } else {
            let stem = Path::new(&path)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            self.task_code = stem.to_lowercase();
            self.task_title = stem.to_uppercase();
        }
    }

    pub fn check_staging_files(&mut self) {
        let staging_dir = AppSettings::staging_dir();
        if staging_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(staging_dir) {
                for e in entries.flatten() {
                    let p = e.path();
                    if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                        if ext.eq_ignore_ascii_case("pdf") && self.pdf_path.is_empty() {
                            self.set_pdf(p.to_string_lossy().to_string());
                        } else if ext.eq_ignore_ascii_case("zip") && self.zip_path.is_empty() {
                            self.zip_path = p.to_string_lossy().to_string();
                        }
                    }
                }
            }
        }
    }

    pub fn set_status(&mut self, msg: &str, is_err: bool) {
        self.status_message = Some((msg.to_string(), is_err));
    }

    pub fn refresh_problems(&mut self) {
        let ws_dir = self.settings.get_workspace_path();
        self.problems = WorkspaceManager::list_problems(&ws_dir);
    }
}

impl eframe::App for AlgoDeckGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Periodic check for daemon alive if in starting state
        if self.is_starting_daemon {
            if Self::check_backend_alive() {
                self.is_backend_online = true;
                self.is_starting_daemon = false;
                self.active_problem_id = Self::fetch_active_problem_id();
                self.refresh_problems();
                self.set_status("AlgoDeck został pomyślnie połączony!", false);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(300));
        }

        // Render main view
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(COLOR_BG_DARK))
            .show(ctx, |ui| {
                if !self.is_backend_online && !self.show_settings_offline {
                    self.render_offline_hero_view(ui);
                } else {
                    self.render_active_dashboard_view(ui);
                }
            });
    }
}

impl AlgoDeckGuiApp {
    // ----------------- 1. OFFLINE HERO VIEW (Approved Emerald Circle) -----------------
    fn render_offline_hero_view(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(35.0);

            // 1. Tytuł i minimalistyczny branding
            ui.label(RichText::new("⚡").color(COLOR_CYAN_ACCENT).size(38.0));
            ui.add_space(6.0);
            ui.label(RichText::new("AlgoDeck").color(Color32::WHITE).size(22.0).strong());
            ui.add_space(2.0);
            ui.label(RichText::new("KOŁO MAP • OLIMPIADA INFORMATYCZNA").color(COLOR_CYAN_ACCENT).size(9.5).strong());

            ui.add_space(38.0);

            // 2. JEDEN DUŻY PRZYCISK W KSZTAŁCIE OKRĘGU (168px Nasycony Szmaragd)
            let diameter = 168.0;
            let (rect, response) = ui.allocate_exact_size(vec2(diameter, diameter), egui::Sense::click());
            let center = rect.center();
            let radius = diameter / 2.0;
            let is_hovered = response.hovered() && !self.is_starting_daemon;

            // Pierścień poświaty (Cyber Emerald Halo Glow)
            if !self.is_starting_daemon {
                let halo_alpha = if is_hovered { 75 } else { 30 };
                let halo_radius = if is_hovered { radius + 11.0 } else { radius + 7.0 };
                ui.painter().circle_stroke(
                    center,
                    halo_radius,
                    Stroke::new(2.5f32, Color32::from_rgba_premultiplied(16, 215, 125, halo_alpha)),
                );
            }

            // Kolory głównego okręgu (Nasycony Szmaragd)
            let (fill_color, border_stroke) = if self.is_starting_daemon {
                (
                    Color32::from_rgb(30, 41, 59),
                    Stroke::new(2.5f32, Color32::from_rgb(71, 85, 105)),
                )
            } else if is_hovered {
                (
                    COLOR_EMERALD_ACCENT,
                    Stroke::new(3.5f32, Color32::from_rgb(209, 250, 229)),
                )
            } else {
                (
                    COLOR_EMERALD,
                    Stroke::new(3.0f32, Color32::from_rgb(52, 235, 155)),
                )
            };

            ui.painter().circle_filled(center, radius, fill_color);
            ui.painter().circle_stroke(center, radius, border_stroke);

            let icon_color = if self.is_starting_daemon { Color32::from_rgb(148, 163, 184) } else { Color32::WHITE };
            let text_color = if self.is_starting_daemon { Color32::from_rgb(148, 163, 184) } else { Color32::from_rgb(236, 253, 245) };
            let label_text = if self.is_starting_daemon { "STARTING..." } else { "START" };

            // Duża błyskawica ⚡
            let bolt_pos = pos2(center.x, center.y - 12.0);
            ui.painter().text(bolt_pos, Align2::CENTER_CENTER, "⚡", egui::FontId::proportional(54.0), icon_color);

            // Podpis START pod błyskawicą
            let label_pos = pos2(center.x, center.y + 32.0);
            ui.painter().text(label_pos, Align2::CENTER_CENTER, label_text, egui::FontId::proportional(12.5), text_color);

            if response.clicked() && !self.is_starting_daemon {
                self.is_starting_daemon = true;
                // Spawns daemon
                let _ = Command::new("algodeck").arg("daemon").spawn();
                self.set_status("Uruchamianie AlgoDeck i Stream Decka w tle...", false);
            }
        });

        // 3. NA SAMYM DOLE: Konfiguracja i Ustawienia
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            ui.add_space(24.0);

            let btn_settings = ui.add(
                egui::Button::new(RichText::new("⚙ Konfiguracja i Ustawienia").color(COLOR_TEXT_MUTED).size(11.5))
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
            );
            if btn_settings.clicked() {
                self.show_settings_offline = true;
                self.current_tab = ActiveTab::Settings;
            }

            if let Some((ref msg, is_err)) = self.status_message {
                let color = if is_err { COLOR_RED } else { COLOR_GREEN };
                ui.label(RichText::new(msg).color(color).size(10.5));
                ui.add_space(4.0);
            }
        });
    }

    // ----------------- 2. ACTIVE DASHBOARD VIEW -----------------
    fn render_active_dashboard_view(&mut self, ui: &mut Ui) {
        // Górny pasek nagłówka
        self.render_header(ui);

        // Pasek aktywnego zadania (jeśli jest zadanie)
        if !self.active_problem_id.is_empty() {
            self.render_active_task_banner(ui);
        }

        ui.add_space(6.0);

        // Nawigacja po zakładkach
        self.render_nav_tabs(ui);

        ui.add_space(8.0);

        // Zawartość aktywnej zakładki
        match self.current_tab {
            ActiveTab::Import => self.render_import_tab(ui),
            ActiveTab::Manual => self.render_manual_tab(ui),
            ActiveTab::Tasks => self.render_tasks_tab(ui),
            ActiveTab::Settings => self.render_settings_tab(ui),
        }

        // Dolny toast
        if let Some((ref msg, is_err)) = self.status_message {
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                ui.add_space(10.0);
                let bg = if is_err { Color32::from_rgb(69, 10, 10) } else { Color32::from_rgb(6, 78, 59) };
                let stroke = if is_err { COLOR_RED } else { COLOR_GREEN };
                egui::Frame::none()
                    .fill(bg)
                    .stroke(Stroke::new(1.0_f32, stroke))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 6.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).color(Color32::WHITE).size(11.0).strong());
                    });
            });
        }
    }

    fn render_header(&mut self, ui: &mut Ui) {
        egui::Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚡").color(COLOR_CYAN_ACCENT).size(18.0));
                    ui.add_space(2.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("AlgoDeck").color(Color32::WHITE).size(13.0).strong());
                        ui.label(RichText::new("KOŁO MAP • OLIMPIADA INFORMATYCZNA").color(COLOR_CYAN_ACCENT).size(8.5).strong());
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (dot_color, label) = if self.is_backend_online {
                            (COLOR_GREEN, "ONLINE")
                        } else {
                            (COLOR_TEXT_MUTED, "OFFLINE")
                        };

                        egui::Frame::none()
                            .fill(Color32::from_rgb(14, 22, 38))
                            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                            .rounding(Rounding::same(12.0))
                            .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.painter().circle_filled(ui.cursor().min + vec2(4.0, 6.0), 3.5, dot_color);
                                    ui.add_space(8.0);
                                    ui.label(RichText::new(label).color(Color32::WHITE).size(9.5).strong());
                                });
                            });
                    });
                });
            });
    }

    fn render_active_task_banner(&mut self, ui: &mut Ui) {
        egui::Frame::none()
            .fill(Color32::from_rgb(14, 30, 52))
            .stroke(Stroke::new(1.0_f32, COLOR_CYAN))
            .inner_margin(egui::Margin::symmetric(14.0, 7.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").color(COLOR_EMERALD_ACCENT).size(11.0));
                    ui.add_space(2.0);
                    ui.label(RichText::new("Aktywne:").color(COLOR_TEXT_MUTED).size(10.5));
                    ui.label(RichText::new(self.active_problem_id.to_uppercase()).color(Color32::WHITE).size(11.5).strong());

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let btn_vs = ui.add(
                            egui::Button::new(RichText::new("VS Code").color(Color32::WHITE).size(10.5).strong())
                                .fill(COLOR_CYAN)
                                .stroke(Stroke::NONE)
                                .rounding(Rounding::same(4.0)),
                        );
                        if btn_vs.clicked() {
                            let _ = ureq::post(&format!("http://127.0.0.1:8080/api/open-vscode/{}", self.active_problem_id)).call();
                        }
                    });
                });
            });
    }

    fn render_nav_tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (ActiveTab::Import, "⚡ Import (PDF + ZIP)"),
                (ActiveTab::Manual, "✍ Ręczny Workspace"),
                (ActiveTab::Tasks, "📁 Zadania"),
                (ActiveTab::Settings, "⚙ Ustawienia"),
            ];

            for (tab, label) in tabs {
                let is_active = self.current_tab == tab;
                let text_color = if is_active { Color32::WHITE } else { COLOR_TEXT_MUTED };
                let bg_color = if is_active { COLOR_SURFACE_LIGHT } else { Color32::TRANSPARENT };
                let stroke = if is_active { Stroke::new(1.0_f32, COLOR_CYAN_ACCENT) } else { Stroke::NONE };

                let btn = ui.add(
                    egui::Button::new(RichText::new(label).color(text_color).size(11.0).strong())
                        .fill(bg_color)
                        .stroke(stroke)
                        .rounding(Rounding::same(5.0)),
                );
                if btn.clicked() {
                    self.current_tab = tab;
                    if tab == ActiveTab::Tasks {
                        self.refresh_problems();
                    }
                }
            }
        });
    }

    // ----------------- TAB 1: IMPORT (PDF + ZIP) -----------------
    fn render_import_tab(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.add_space(4.0);

            // Grid z dwoma obszarami dropzone
            ui.columns(2, |cols| {
                // Lewa kolumna: Plik PDF
                cols[0].group(|ui| {
                    ui.label(RichText::new("📄 Treść zadania (PDF)").color(Color32::WHITE).size(11.0).strong());
                    ui.add_space(4.0);

                    if self.pdf_path.is_empty() {
                        ui.label(RichText::new("Wybierz lub upuść plik .pdf").color(COLOR_TEXT_MUTED).size(10.0));
                        ui.add_space(6.0);
                        if ui.button("Wybierz PDF...").clicked() {
                            if let Some(path) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file() {
                                self.set_pdf(path.to_string_lossy().to_string());
                            }
                        }
                    } else {
                        let fname = Path::new(&self.pdf_path).file_name().unwrap_or_default().to_string_lossy().to_string();
                        ui.label(RichText::new(format!("✓ {}", fname)).color(COLOR_EMERALD_ACCENT).size(10.5).strong());
                        ui.add_space(4.0);
                        if ui.button("✕ Usuń").clicked() {
                            self.pdf_path.clear();
                            self.task_code.clear();
                            self.task_title.clear();
                        }
                    }
                });

                // Prawa kolumna: Plik ZIP
                cols[1].group(|ui| {
                    ui.label(RichText::new("📦 Paczka testów (ZIP)").color(Color32::WHITE).size(11.0).strong());
                    ui.add_space(4.0);

                    if self.zip_path.is_empty() {
                        ui.label(RichText::new("Wybierz lub upuść plik .zip").color(COLOR_TEXT_MUTED).size(10.0));
                        ui.add_space(6.0);
                        if ui.button("Wybierz ZIP...").clicked() {
                            if let Some(path) = rfd::FileDialog::new().add_filter("ZIP", &["zip"]).pick_file() {
                                self.zip_path = path.to_string_lossy().to_string();
                                self.set_status("Dołączono archiwum ZIP!", false);
                            }
                        }
                    } else {
                        let fname = Path::new(&self.zip_path).file_name().unwrap_or_default().to_string_lossy().to_string();
                        ui.label(RichText::new(format!("✓ {}", fname)).color(COLOR_EMERALD_ACCENT).size(10.5).strong());
                        ui.add_space(4.0);
                        if ui.button("✕ Usuń").clicked() {
                            self.zip_path.clear();
                        }
                    }
                });
            });

            ui.add_space(8.0);

            // Karta z weryfikacją metadanych (jeśli wybrano PDF lub podano kod)
            if !self.pdf_path.is_empty() || !self.task_code.is_empty() {
                egui::Frame::none()
                    .fill(COLOR_SURFACE)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Kod:").color(COLOR_TEXT_MUTED).size(10.5));
                            ui.add_sized(vec2(70.0, 22.0), egui::TextEdit::singleline(&mut self.task_code));

                            ui.add_space(8.0);
                            ui.label(RichText::new("Tytuł:").color(COLOR_TEXT_MUTED).size(10.5));
                            ui.add_sized(vec2(ui.available_width() - 10.0, 22.0), egui::TextEdit::singleline(&mut self.task_title));
                        });

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Limit czasu (s):").color(COLOR_TEXT_MUTED).size(10.5));
                            ui.add(egui::DragValue::new(&mut self.time_limit).speed(0.1).range(0.1..=10.0));

                            ui.add_space(12.0);
                            ui.label(RichText::new("RAM (MB):").color(COLOR_TEXT_MUTED).size(10.5));
                            ui.add(egui::DragValue::new(&mut self.memory_limit).speed(16).range(16..=2048));
                        });

                        if self.detected_tests_count > 0 {
                            ui.add_space(4.0);
                            ui.label(RichText::new(format!("● Wykryto {} oficjalnych testów w PDF", self.detected_tests_count)).color(COLOR_EMERALD_ACCENT).size(10.0));
                        }
                    });

                ui.add_space(10.0);

                // Duży przycisk rozpoczęcia
                let btn_start = ui.add_sized(
                    vec2(ui.available_width(), 38.0),
                    egui::Button::new(RichText::new("⚡ Rozpocznij Zadanie w VS Code").color(Color32::WHITE).size(12.5).strong())
                        .fill(COLOR_EMERALD)
                        .stroke(Stroke::NONE)
                        .rounding(Rounding::same(6.0)),
                );

                if btn_start.clicked() && !self.is_submitting {
                    self.submit_import_task();
                }
            }
        });
    }

    fn submit_import_task(&mut self) {
        let code = self.task_code.trim().to_lowercase();
        if code.is_empty() {
            self.set_status("Podaj kod zadania!", true);
            return;
        }

        self.is_submitting = true;
        let _ws_dir = self.settings.get_workspace_path();

        let payload = serde_json::json!({
            "file_path": self.pdf_path,
            "problem_id": code,
            "title": self.task_title,
            "time_limit": self.time_limit,
            "memory_limit": self.memory_limit,
            "auto_create": true
        });

        match ureq::post("http://127.0.0.1:8080/api/auto-import-pdf")
            .timeout(std::time::Duration::from_secs(6))
            .send_json(payload)
        {
            Ok(_) => {
                if !self.zip_path.is_empty() {
                    let _ = self.upload_zip_tests(&code);
                }
                self.active_problem_id = code.clone();
                self.set_status(&format!("Zadanie {} utworzone!", code.to_uppercase()), false);
                self.pdf_path.clear();
                self.zip_path.clear();
                self.task_code.clear();
                self.task_title.clear();
                self.refresh_problems();
            }
            Err(e) => {
                self.set_status(&format!("Błąd: {}", e), true);
            }
        }
        self.is_submitting = false;
    }

    fn upload_zip_tests(&self, problem_id: &str) -> Result<(), String> {
        let zip_bytes = std::fs::read(&self.zip_path).map_err(|e| e.to_string())?;
        let ws_dir = self.settings.get_workspace_path();
        WorkspaceManager::add_tests_from_zip(&ws_dir, problem_id, &zip_bytes).map(|_| ())
    }

    // ----------------- TAB 2: RĘCZNY WORKSPACE -----------------
    fn render_manual_tab(&mut self, ui: &mut Ui) {
        egui::Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .rounding(Rounding::same(6.0))
            .inner_margin(egui::Margin::symmetric(14.0, 12.0))
            .show(ui, |ui| {
                ui.label(RichText::new("Kod zadania * (np. chw, kol, sil):").color(COLOR_TEXT_MUTED).size(10.5));
                ui.add_sized(vec2(ui.available_width(), 24.0), egui::TextEdit::singleline(&mut self.task_code));

                ui.add_space(6.0);
                ui.label(RichText::new("Pełny tytuł (opcjonalny):").color(COLOR_TEXT_MUTED).size(10.5));
                ui.add_sized(vec2(ui.available_width(), 24.0), egui::TextEdit::singleline(&mut self.task_title));

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Czas (s):").color(COLOR_TEXT_MUTED).size(10.5));
                    ui.add(egui::DragValue::new(&mut self.time_limit).speed(0.1).range(0.1..=10.0));
                    ui.add_space(8.0);
                    ui.label(RichText::new("RAM (MB):").color(COLOR_TEXT_MUTED).size(10.5));
                    ui.add(egui::DragValue::new(&mut self.memory_limit).speed(16).range(16..=2048));
                });

                ui.add_space(6.0);
                ui.label(RichText::new("Przykładowe wejście cin:").color(COLOR_TEXT_MUTED).size(10.5));
                ui.add_sized(vec2(ui.available_width(), 44.0), egui::TextEdit::multiline(&mut self.manual_cin));

                ui.add_space(4.0);
                ui.label(RichText::new("Oczekiwane wyjście cout:").color(COLOR_TEXT_MUTED).size(10.5));
                ui.add_sized(vec2(ui.available_width(), 44.0), egui::TextEdit::multiline(&mut self.manual_cout));

                ui.add_space(10.0);
                let btn_create = ui.add_sized(
                    vec2(ui.available_width(), 34.0),
                    egui::Button::new(RichText::new("Utwórz Workspace").color(Color32::WHITE).size(12.0).strong())
                        .fill(COLOR_EMERALD)
                        .stroke(Stroke::NONE)
                        .rounding(Rounding::same(5.0)),
                );

                if btn_create.clicked() {
                    let code = self.task_code.trim().to_lowercase();
                    if code.is_empty() {
                        self.set_status("Podaj kod zadania!", true);
                        return;
                    }
                    let payload = serde_json::json!({
                        "problem_id": code,
                        "title": self.task_title,
                        "time_limit": self.time_limit,
                        "memory_limit": self.memory_limit,
                        "sample_cin": self.manual_cin,
                        "sample_cout": self.manual_cout
                    });
                    if let Ok(_) = ureq::post("http://127.0.0.1:8080/api/create-manual").send_json(payload) {
                        self.active_problem_id = code.clone();
                        self.set_status(&format!("Utworzono zadanie {}", code.to_uppercase()), false);
                        self.task_code.clear();
                        self.task_title.clear();
                        self.manual_cin.clear();
                        self.manual_cout.clear();
                        self.refresh_problems();
                    }
                }
            });
    }

    // ----------------- TAB 3: ZADANIA (GALLERY) -----------------
    fn render_tasks_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🔍").size(12.0));
            ui.add_sized(vec2(ui.available_width() - 80.0, 24.0), egui::TextEdit::singleline(&mut self.search_query).hint_text("Szukaj zadania..."));
            if ui.button("Odśwież").clicked() {
                self.refresh_problems();
            }
        });

        ui.add_space(8.0);

        let query = self.search_query.to_lowercase();
        let filtered: Vec<ProblemSummary> = self.problems.iter().filter(|p| {
            query.is_empty() || p.problem_id.contains(&query) || p.title.to_lowercase().contains(&query)
        }).cloned().collect();

        if filtered.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.label(RichText::new("Brak zadań w katalogu workspace").color(COLOR_TEXT_MUTED).size(12.0));
            });
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for prob in filtered {
                let is_active = prob.problem_id == self.active_problem_id;
                let border = if is_active {
                    Stroke::new(1.5_f32, COLOR_EMERALD_ACCENT)
                } else {
                    Stroke::new(1.0_f32, COLOR_BORDER)
                };

                egui::Frame::none()
                    .fill(COLOR_SURFACE)
                    .stroke(border)
                    .rounding(Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Badge z kodem
                            egui::Frame::none()
                                .fill(Color32::from_rgb(14, 30, 52))
                                .stroke(Stroke::new(1.0_f32, COLOR_CYAN))
                                .rounding(Rounding::same(4.0))
                                .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(prob.problem_id.to_uppercase()).color(COLOR_CYAN_ACCENT).size(10.5).strong());
                                });

                            ui.add_space(4.0);
                            ui.label(RichText::new(&prob.title).color(Color32::WHITE).size(11.5).strong());

                            ui.add_space(4.0);
                            ui.label(RichText::new(format!("({} testów)", prob.tests_count)).color(COLOR_TEXT_MUTED).size(10.0));

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if !is_active {
                                    let btn_act = ui.add(
                                        egui::Button::new(RichText::new("⚡ Aktywuj").color(Color32::WHITE).size(10.0).strong())
                                            .fill(COLOR_EMERALD)
                                            .stroke(Stroke::NONE)
                                            .rounding(Rounding::same(4.0)),
                                    );
                                    if btn_act.clicked() {
                                        let _ = ureq::post(&format!("http://127.0.0.1:8080/api/set-active/{}", prob.problem_id)).call();
                                        self.active_problem_id = prob.problem_id.clone();
                                    }
                                }

                                let btn_code = ui.add(
                                    egui::Button::new(RichText::new("VS Code").color(Color32::WHITE).size(10.0))
                                        .fill(Color32::from_rgb(14, 30, 52))
                                        .stroke(Stroke::new(1.0_f32, COLOR_CYAN))
                                        .rounding(Rounding::same(4.0)),
                                );
                                if btn_code.clicked() {
                                    let _ = ureq::post(&format!("http://127.0.0.1:8080/api/open-vscode/{}", prob.problem_id)).call();
                                }
                            });
                        });
                    });

                ui.add_space(4.0);
            }
        });
    }

    // ----------------- TAB 4: USTAWIENIA (Clean, Clear, Highly Readable) -----------------
    fn render_settings_tab(&mut self, ui: &mut Ui) {
        egui::Frame::none()
            .fill(COLOR_SURFACE)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .rounding(Rounding::same(6.0))
            .inner_margin(egui::Margin::symmetric(14.0, 12.0))
            .show(ui, |ui| {
                ui.label(RichText::new("📁 ŚRODOWISKO I KATALOGI").color(COLOR_CYAN_ACCENT).size(11.0).strong());
                ui.add_space(4.0);

                ui.label(RichText::new("Główny folder z zadaniami olimpijskimi:").color(COLOR_TEXT_MUTED).size(10.5));
                ui.horizontal(|ui| {
                    ui.add_sized(vec2(ui.available_width() - 85.0, 24.0), egui::TextEdit::singleline(&mut self.settings.workspace_dir));
                    if ui.button("Zmień...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            self.settings.workspace_dir = folder.to_string_lossy().to_string();
                        }
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(RichText::new("⚡ KOMPILATOR C++").color(COLOR_CYAN_ACCENT).size(11.0).strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Standard:").color(COLOR_TEXT_MUTED).size(10.5));
                    for std in ["c++17", "c++20", "c++23"] {
                        if ui.selectable_label(self.settings.cpp_standard == std, std.to_uppercase()).clicked() {
                            self.settings.cpp_standard = std.to_string();
                        }
                    }
                });

                ui.add_space(4.0);
                ui.label(RichText::new("Flagi optymalizacji (Release):").color(COLOR_TEXT_MUTED).size(10.0));
                ui.add_sized(vec2(ui.available_width(), 22.0), egui::TextEdit::singleline(&mut self.settings.cxx_release_flags));

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(RichText::new("🛠 OPCJE EDYTORA I INTEGRACJI").color(COLOR_CYAN_ACCENT).size(11.0).strong());
                ui.add_space(4.0);

                ui.checkbox(&mut self.settings.open_pdf_in_vscode, "Dziel ekran w VS Code (Kod po lewej, PDF po prawej)");
                ui.checkbox(&mut self.settings.notifications, "Powiadomienia na pulpicie o wyniku kompilacji i testów");
                ui.checkbox(&mut self.settings.auto_detect_downloads, "Automatycznie wykrywaj pobrane zadania ze Szkopuła i OIJ");

                ui.add_space(12.0);

                let btn_save = ui.add_sized(
                    vec2(ui.available_width(), 34.0),
                    egui::Button::new(RichText::new("💾 Zapisz Ustawienia").color(Color32::WHITE).size(11.5).strong())
                        .fill(COLOR_CYAN)
                        .stroke(Stroke::NONE)
                        .rounding(Rounding::same(5.0)),
                );

                if btn_save.clicked() {
                    match self.settings.save() {
                        Ok(_) => {
                            let _ = ureq::post("http://127.0.0.1:8080/api/settings").send_json(serde_json::to_value(&self.settings).unwrap());
                            self.set_status("Ustawienia zapisane pomyślnie!", false);
                            self.show_settings_offline = false;
                        }
                        Err(e) => {
                            self.set_status(&format!("Błąd zapisu: {}", e), true);
                        }
                    }
                }
            });
    }
}
