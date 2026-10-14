use egui::{pos2, vec2, Align, Align2, Color32, Layout, RichText, Rounding, Stroke, Ui};
use std::path::Path;
use std::process::Command;

use crate::config::AppSettings;
use crate::gui::theme::*;
use crate::pdf::PdfParser;
use crate::workspace::{ProblemSummary, WorkspaceManager};

#[derive(PartialEq, Clone, Copy)]
pub enum ActiveTab {
    NewTask,
    TasksList,
    Settings,
}

pub struct AlgoDeckGuiApp {
    pub current_tab: ActiveTab,
    pub settings: AppSettings,

    // Import / New task state
    pub pdf_path: String,
    pub zip_path: String,
    pub task_code: String,
    pub task_title: String,
    pub time_limit: f64,
    pub memory_limit: u64,
    pub detected_tests_count: usize,
    pub is_submitting: bool,

    // Manual input toggle
    pub show_manual_inputs: bool,
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
    pub status_message: Option<(String, bool)>,
}

impl AlgoDeckGuiApp {
    pub fn new(initial_pdf: Option<String>) -> Self {
        let settings = AppSettings::load();
        let ws_dir = settings.get_workspace_path();
        let problems = WorkspaceManager::list_problems(&ws_dir);

        let is_online = Self::check_backend_alive();
        let active_id = Self::fetch_active_problem_id();

        let mut app = Self {
            current_tab: ActiveTab::NewTask,
            settings,
            pdf_path: String::new(),
            zip_path: String::new(),
            task_code: String::new(),
            task_title: String::new(),
            time_limit: 1.0,
            memory_limit: 128,
            detected_tests_count: 0,
            is_submitting: false,
            show_manual_inputs: false,
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
            app.check_staging_files();
        }

        app
    }

    fn check_backend_alive() -> bool {
        match ureq::get("http://127.0.0.1:8080/api/status")
            .timeout(std::time::Duration::from_millis(300))
            .call()
        {
            Ok(r) => r.status() == 200,
            Err(_) => false,
        }
    }

    fn fetch_active_problem_id() -> String {
        if let Ok(res) = ureq::get("http://127.0.0.1:8080/api/status")
            .timeout(std::time::Duration::from_millis(300))
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
        // Okresowe sprawdzanie czy serwer jest online podczas startu
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

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(COLOR_BG_DARK))
            .show(ctx, |ui| {
                // 1. Oryginalny, customowy pasek tytułowy (z obsługą przeciągania, minimalizacji i zamykania)
                self.render_custom_titlebar(ctx, ui);

                // 2. Widok Standby Hero (gdy backend offline i nie przeglądamy ustawień)
                if !self.is_backend_online && !self.show_settings_offline {
                    self.render_offline_hero_view(ui);
                    return;
                }

                // 3. Pasek Zakładek (Nowe Zadanie, Zadania, Ustawienia)
                self.render_tab_bar(ui);

                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);

                // 4. Zawartość aktywnej zakładki w przewijanym obszarze
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                        match self.current_tab {
                            ActiveTab::NewTask => self.render_tab_new_task(ui),
                            ActiveTab::TasksList => self.render_tab_tasks(ui),
                            ActiveTab::Settings => self.render_tab_settings(ui),
                        }
                    });

                // 5. Dolny pasek statusu (Toast)
                if let Some((ref msg, is_err)) = self.status_message {
                    ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                        let fill = if is_err { Color32::from_rgb(127, 29, 29) } else { Color32::from_rgb(6, 78, 59) };
                        let stroke = if is_err { COLOR_RED } else { COLOR_GREEN };
                        egui::Frame::none()
                            .fill(fill)
                            .stroke(Stroke::new(1.0f32, stroke))
                            .rounding(Rounding::same(5.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 5.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new(msg).color(Color32::WHITE).size(11.0).strong());
                            });
                    });
                }
            });
    }
}

impl AlgoDeckGuiApp {
    // ----------------- 1. ORYGINALNY CUSTOMOWY PASEK TYTUŁOWY -----------------
    fn render_custom_titlebar(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        let width = ui.available_width();
        egui::Frame::none()
            .fill(Color32::from_rgb(20, 24, 34))
            .rounding(0.0)
            .inner_margin(0.0)
            .show(ui, |ui| {
                ui.set_min_size(vec2(width, 34.0));
                ui.set_max_size(vec2(width, 34.0));

                let title_bar_rect = ui.max_rect();

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    let (close_rect, min_rect) = ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let button_size = [42.0, 34.0];
                        let stroke = Stroke::new(1.0f32, Color32::from_gray(215));

                        // 1. Zamknij (X)
                        let (close_rect, close_resp) = ui.allocate_exact_size(button_size.into(), egui::Sense::click());
                        if close_resp.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        let is_close_hovered = close_resp.hovered()
                            || ctx.input(|i| i.pointer.hover_pos().is_some_and(|p| close_rect.contains(p)));
                        if is_close_hovered {
                            ui.painter().rect_filled(close_rect, 0.0, Color32::from_rgb(200, 40, 40));
                        }
                        let center = close_rect.center();
                        ui.painter().line_segment([center + vec2(-4.0, -4.0), center + vec2(4.0, 4.0)], stroke);
                        ui.painter().line_segment([center + vec2(-4.0, 4.0), center + vec2(4.0, -4.0)], stroke);
                        close_resp.on_hover_text("Zamknij");

                        // 2. Minimalizuj (-)
                        let (min_rect, min_resp) = ui.allocate_exact_size(button_size.into(), egui::Sense::click());
                        if min_resp.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        let is_min_hovered = min_resp.hovered()
                            || ctx.input(|i| i.pointer.hover_pos().is_some_and(|p| min_rect.contains(p)));
                        if is_min_hovered {
                            ui.painter().rect_filled(min_rect, 0.0, Color32::from_white_alpha(15));
                        }
                        let m_center = min_rect.center();
                        ui.painter().line_segment([m_center + vec2(-4.0, 0.0), m_center + vec2(4.0, 0.0)], stroke);
                        min_resp.on_hover_text("Minimalizuj");

                        // 3. Lewa strona (tytuł i logo)
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.add_space(10.0);
                            ui.label(RichText::new("⚡").color(COLOR_CYAN_ACCENT).size(14.0).strong());
                            ui.add_space(4.0);
                            ui.label(RichText::new("AlgoDeck").color(Color32::from_gray(230)).size(12.0).strong());
                            ui.add_space(6.0);
                            ui.label(RichText::new("KOŁO MAP • OI").color(COLOR_CYAN_ACCENT).size(9.5).strong());
                        });

                        (close_rect, min_rect)
                    }).inner;

                    // Przeciąganie okna kursorem myszy
                    if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
                        if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
                            if title_bar_rect.contains(pos) && !close_rect.contains(pos) && !min_rect.contains(pos) {
                                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                            }
                        }
                    }
                });
            });
    }

    // ----------------- 2. PASEK ZAKŁADEK -----------------
    fn render_tab_bar(&mut self, ui: &mut Ui) {
        egui::Frame::none()
            .fill(Color32::from_rgb(14, 18, 28))
            .inner_margin(egui::Margin::symmetric(10.0, 4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let tabs = [
                        (ActiveTab::NewTask, "+ Nowe Zadanie"),
                        (ActiveTab::TasksList, "Zadania"),
                        (ActiveTab::Settings, "Ustawienia"),
                    ];

                    for (tab, label) in tabs {
                        let is_active = self.current_tab == tab;
                        let bg = if is_active { COLOR_SURFACE_LIGHT } else { Color32::TRANSPARENT };
                        let fg = if is_active { COLOR_CYAN_ACCENT } else { COLOR_TEXT_MUTED };

                        let btn = ui.add(
                            egui::Button::new(RichText::new(label).color(fg).size(12.0).strong())
                                .fill(bg)
                                .stroke(Stroke::NONE)
                                .rounding(Rounding::same(4.0))
                                .min_size(vec2(85.0, 24.0)),
                        );

                        if btn.clicked() {
                            self.current_tab = tab;
                            self.show_settings_offline = false;
                            if tab == ActiveTab::TasksList {
                                self.refresh_problems();
                            }
                        }
                    }

                    // Wskaźnik ONLINE po prawej stronie
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (dot_color, label) = if self.is_backend_online {
                            (COLOR_GREEN, "ONLINE")
                        } else {
                            (COLOR_TEXT_MUTED, "OFFLINE")
                        };
                        ui.label(RichText::new(label).color(dot_color).size(10.0).strong());
                    });
                });
            });
    }

    // ----------------- 3. OFFLINE HERO VIEW (SZMARAGDOWY OKRĄGŁY PRZYCISK) -----------------
    fn render_offline_hero_view(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(28.0);

            ui.label(RichText::new("⚡").color(COLOR_CYAN_ACCENT).size(36.0));
            ui.add_space(4.0);
            ui.label(RichText::new("AlgoDeck").color(Color32::WHITE).size(20.0).strong());
            ui.add_space(2.0);
            ui.label(RichText::new("KOŁO MAP • OLIMPIADA INFORMATYCZNA").color(COLOR_CYAN_ACCENT).size(9.0).strong());

            ui.add_space(32.0);

            // JEDEN DUŻY PRZYCISK W KSZTAŁCIE OKRĘGU (168px Nasycony Szmaragd)
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

            // Kolory głównego okręgu
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

            let bolt_pos = pos2(center.x, center.y - 12.0);
            ui.painter().text(bolt_pos, Align2::CENTER_CENTER, "⚡", egui::FontId::proportional(54.0), icon_color);

            let label_pos = pos2(center.x, center.y + 32.0);
            ui.painter().text(label_pos, Align2::CENTER_CENTER, label_text, egui::FontId::proportional(12.5), text_color);

            if response.clicked() && !self.is_starting_daemon {
                self.is_starting_daemon = true;
                #[cfg(target_os = "linux")]
                {
                    let _ = Command::new("systemctl").args(["--user", "start", "algodeck.service"]).spawn();
                }
                let self_exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("algodeck"));
                let _ = Command::new(self_exe).arg("daemon").spawn();
                self.set_status("Uruchamianie AlgoDeck i Stream Decka w tle...", false);
            }
        });

        // Konfiguracja i Ustawienia na dole
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            ui.add_space(20.0);
            let btn_settings = ui.add(
                egui::Button::new(RichText::new("⚙ Konfiguracja i Ustawienia").color(COLOR_TEXT_MUTED).size(11.0))
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
            );
            if btn_settings.clicked() {
                self.show_settings_offline = true;
                self.current_tab = ActiveTab::Settings;
            }
        });
    }

    // ----------------- 4. TAB 1: NOWE ZADANIE -----------------
    fn render_tab_new_task(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Karta 1: Treść zadania (PDF)
            egui::Frame::none()
                .fill(COLOR_SURFACE)
                .rounding(Rounding::same(6.0))
                .stroke(Stroke::new(1.0f32, COLOR_BORDER))
                .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("📄 Treść zadania (PDF)").color(COLOR_CYAN_ACCENT).size(11.0).strong());
                    ui.add_space(4.0);

                    if self.pdf_path.is_empty() {
                        let btn = ui.add_sized(
                            vec2(ui.available_width(), 30.0),
                            egui::Button::new(RichText::new("Wybierz plik PDF...").size(11.0)),
                        );
                        if btn.clicked() {
                            if let Some(file) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file() {
                                self.set_pdf(file.to_string_lossy().to_string());
                            }
                        }
                    } else {
                        let name = Path::new(&self.pdf_path)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&name).color(Color32::WHITE).size(11.0));
                            if ui.button(RichText::new("Usuń").color(COLOR_RED).size(10.0)).clicked() {
                                self.pdf_path.clear();
                            }
                        });
                    }
                });

            ui.add_space(4.0);

            // Karta 2: Paczka testów (ZIP)
            egui::Frame::none()
                .fill(COLOR_SURFACE)
                .rounding(Rounding::same(6.0))
                .stroke(Stroke::new(1.0f32, COLOR_BORDER))
                .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("📦 Paczka testów (ZIP - opcjonalnie)").color(COLOR_CYAN_ACCENT).size(11.0).strong());
                    ui.add_space(4.0);

                    if self.zip_path.is_empty() {
                        let btn = ui.add_sized(
                            vec2(ui.available_width(), 30.0),
                            egui::Button::new(RichText::new("Wybierz plik ZIP...").size(11.0)),
                        );
                        if btn.clicked() {
                            if let Some(file) = rfd::FileDialog::new().add_filter("ZIP", &["zip"]).pick_file() {
                                self.zip_path = file.to_string_lossy().to_string();
                            }
                        }
                    } else {
                        let name = Path::new(&self.zip_path)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&name).color(Color32::WHITE).size(11.0));
                            if ui.button(RichText::new("Usuń").color(COLOR_RED).size(10.0)).clicked() {
                                self.zip_path.clear();
                            }
                        });
                    }
                });

            ui.add_space(4.0);

            // Karta 3: Metadane zadania
            egui::Frame::none()
                .fill(COLOR_SURFACE)
                .rounding(Rounding::same(6.0))
                .stroke(Stroke::new(1.0f32, COLOR_BORDER))
                .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Kod:").color(COLOR_TEXT_MUTED).size(11.0));
                        ui.add_sized(vec2(70.0, 22.0), egui::TextEdit::singleline(&mut self.task_code));

                        ui.add_space(6.0);
                        ui.label(RichText::new("Tytuł:").color(COLOR_TEXT_MUTED).size(11.0));
                        ui.add_sized(vec2(ui.available_width(), 22.0), egui::TextEdit::singleline(&mut self.task_title));
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Limit (s):").color(COLOR_TEXT_MUTED).size(11.0));
                        let mut t_str = format!("{:.1}", self.time_limit);
                        if ui.add_sized(vec2(45.0, 22.0), egui::TextEdit::singleline(&mut t_str)).changed() {
                            if let Ok(v) = t_str.parse::<f64>() {
                                self.time_limit = v;
                            }
                        }

                        ui.add_space(6.0);
                        ui.label(RichText::new("RAM (MB):").color(COLOR_TEXT_MUTED).size(11.0));
                        let mut m_str = format!("{}", self.memory_limit);
                        if ui.add_sized(vec2(55.0, 22.0), egui::TextEdit::singleline(&mut m_str)).changed() {
                            if let Ok(v) = m_str.parse::<u64>() {
                                self.memory_limit = v;
                            }
                        }
                    });

                    ui.add_space(4.0);
                    ui.checkbox(&mut self.show_manual_inputs, "Wklej ręczne testy (cin / cout)");
                    if self.show_manual_inputs {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Wejście (cin):").color(COLOR_TEXT_MUTED).size(9.5));
                                ui.add_sized(vec2(ui.available_width() / 2.0 - 4.0, 48.0), egui::TextEdit::multiline(&mut self.manual_cin));
                            });
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Wyjście (cout):").color(COLOR_TEXT_MUTED).size(9.5));
                                ui.add_sized(vec2(ui.available_width(), 48.0), egui::TextEdit::multiline(&mut self.manual_cout));
                            });
                        });
                    }
                });

            ui.add_space(10.0);

            // Główny przycisk startu zadania
            let can_submit = !self.task_code.trim().is_empty() && !self.is_submitting;
            let start_btn = ui.add_sized(
                vec2(ui.available_width(), 42.0),
                egui::Button::new(RichText::new("⚡ Rozpocznij Zadanie w VS Code").color(Color32::WHITE).size(13.0).strong())
                    .fill(if can_submit { COLOR_EMERALD } else { Color32::from_rgb(30, 41, 59) })
                    .stroke(Stroke::NONE)
                    .rounding(Rounding::same(6.0)),
            );

            if start_btn.clicked() && can_submit {
                self.create_and_launch_task();
            }
        });
    }

    fn create_and_launch_task(&mut self) {
        self.is_submitting = true;
        let ws_dir = self.settings.get_workspace_path();
        let code = self.task_code.trim().to_lowercase();
        let title = if self.task_title.is_empty() { code.to_uppercase() } else { self.task_title.clone() };

        let mut tests = Vec::new();
        if !self.manual_cin.trim().is_empty() {
            tests.push(crate::pdf::TestCase {
                id: "test_1".to_string(),
                name: "Test 1".to_string(),
                input: self.manual_cin.clone(),
                expected_output: self.manual_cout.clone(),
                is_edge_case: false,
                tag: "manual".to_string(),
            });
        }

        let analysis = crate::pdf::ProblemAnalysis {
            problem_id: code.clone(),
            title: title.clone(),
            time_limit_sec: self.time_limit,
            memory_limit_mb: self.memory_limit,
            tests,
            used_engine: "manual".to_string(),
        };

        match WorkspaceManager::create_problem(
            &ws_dir,
            &analysis,
            if self.pdf_path.is_empty() { None } else { Some(Path::new(&self.pdf_path)) },
        ) {
            Ok(_) => {
                if !self.zip_path.is_empty() {
                    if let Ok(bytes) = std::fs::read(&self.zip_path) {
                        let _ = WorkspaceManager::add_tests_from_zip(&ws_dir, &code, &bytes);
                    }
                }
                let _ = ureq::post(&format!("http://127.0.0.1:8080/api/set-active/{}", code)).call();
                let _ = ureq::post(&format!("http://127.0.0.1:8080/api/open-vscode/{}", code)).call();

                self.active_problem_id = code.clone();
                self.set_status(&format!("Zadanie {} utworzone i otwarte w VS Code!", code.to_uppercase()), false);
                self.refresh_problems();
            }
            Err(e) => {
                self.set_status(&format!("Błąd tworzenia zadania: {}", e), true);
            }
        }
        self.is_submitting = false;
    }

    // ----------------- 5. TAB 2: ZADANIA (WIDOK LISTY KTÓRY SIĘ SPODOBAŁ) -----------------
    fn render_tab_tasks(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🔎").size(12.0));
            ui.add_sized(
                vec2(ui.available_width() - 75.0, 24.0),
                egui::TextEdit::singleline(&mut self.search_query).hint_text("Filtruj zadania..."),
            );
            if ui.button(RichText::new("Odśwież").size(10.5)).clicked() {
                self.refresh_problems();
                self.active_problem_id = Self::fetch_active_problem_id();
            }
        });

        ui.add_space(6.0);

        let query = self.search_query.trim().to_lowercase();
        let filtered_problems: Vec<ProblemSummary> = self.problems
            .iter()
            .filter(|p| {
                if query.is_empty() {
                    true
                } else {
                    p.problem_id.to_lowercase().contains(&query)
                        || p.title.to_lowercase().contains(&query)
                }
            })
            .cloned()
            .collect();

        if filtered_problems.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(RichText::new("Brak zadań w katalogu workspace").color(COLOR_TEXT_MUTED).size(12.0));
            });
            return;
        }

        let mut to_delete = None;
        let mut to_set_active = None;
        let mut to_open_vscode = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for prob in &filtered_problems {
                let is_active = self.active_problem_id == prob.problem_id;
                let border_col = if is_active { COLOR_EMERALD } else { COLOR_BORDER };

                egui::Frame::none()
                    .fill(COLOR_SURFACE)
                    .rounding(Rounding::same(6.0))
                    .stroke(Stroke::new(1.0f32, border_col))
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let badge_bg = if is_active { COLOR_EMERALD } else { COLOR_SURFACE_LIGHT };
                            egui::Frame::none()
                                .fill(badge_bg)
                                .rounding(Rounding::same(4.0))
                                .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(prob.problem_id.to_uppercase()).color(Color32::WHITE).size(11.0).strong());
                                });

                            ui.add_space(4.0);
                            ui.label(RichText::new(&prob.title).color(Color32::WHITE).size(12.0).strong());

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new(format!("{} testów", prob.tests_count)).color(COLOR_TEXT_MUTED).size(10.0));
                            });
                        });

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{:.1}s | {}MB", prob.time_limit_sec, prob.memory_limit_mb)).color(COLOR_TEXT_MUTED).size(10.0));

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                // Przycisk Usuń
                                if ui.button(RichText::new("🗑").color(COLOR_RED).size(11.0)).clicked() {
                                    to_delete = Some(prob.problem_id.clone());
                                }

                                // Przycisk VS Code
                                if ui.button(RichText::new("VS Code").size(10.5)).clicked() {
                                    to_open_vscode = Some(prob.problem_id.clone());
                                }

                                // Przycisk Aktywuj
                                if is_active {
                                    ui.label(RichText::new("✓ Aktywne").color(COLOR_EMERALD).size(10.5).strong());
                                } else {
                                    if ui.button(RichText::new("Aktywuj").size(10.5)).clicked() {
                                        to_set_active = Some(prob.problem_id.clone());
                                    }
                                }
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        });

        if let Some(pid) = to_delete {
            let ws_dir = self.settings.get_workspace_path();
            let _ = WorkspaceManager::delete_problem(&ws_dir, &pid);
            self.refresh_problems();
        }
        if let Some(pid) = to_open_vscode {
            let _ = ureq::post(&format!("http://127.0.0.1:8080/api/open-vscode/{}", pid)).call();
        }
        if let Some(pid) = to_set_active {
            let _ = ureq::post(&format!("http://127.0.0.1:8080/api/set-active/{}", pid)).call();
            self.active_problem_id = pid;
        }
    }

    // ----------------- 6. TAB 3: USTAWIENIA (PRZYWRÓCONY ORYGINALNY UKŁAD Z C74952D) -----------------
    fn render_tab_settings(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            // 1. Przycisk zapisu przypięty do SAMEGO DOŁU okna
            ui.add_space(4.0);
            let btn_save = ui.add_sized(
                vec2(ui.available_width(), 36.0),
                egui::Button::new(RichText::new("Zapisz Ustawienia").color(Color32::WHITE).size(12.0).strong())
                    .fill(COLOR_CYAN)
                    .stroke(Stroke::NONE)
                    .rounding(Rounding::same(6.0)),
            );

            if btn_save.clicked() {
                match self.settings.save() {
                    Ok(_) => self.set_status("Ustawienia zapisane pomyślnie!", false),
                    Err(e) => self.set_status(&format!("Błąd zapisu: {}", e), true),
                }
            }

            // 2. Jeden spójny panel ustawień wypełniający całą dostępną przestrzeń
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Frame::none()
                        .fill(COLOR_SURFACE)
                        .rounding(Rounding::same(6.0))
                        .stroke(Stroke::new(1.0f32, COLOR_BORDER))
                        .inner_margin(egui::Margin::symmetric(10.0, 10.0))
                        .show(ui, |ui| {
                            // --- Sekcja 1: Kompilator i Środowisko ---
                            ui.label(RichText::new("Kompilator i Środowisko").color(COLOR_CYAN_ACCENT).size(11.5).strong());
                            ui.add_space(5.0);

                            // 1. Katalog roboczy
                            ui.label(RichText::new("Katalog roboczy (Workspace):").color(COLOR_TEXT_MUTED).size(10.5).strong());
                            ui.horizontal(|ui| {
                                ui.add_sized(vec2(ui.available_width() - 65.0, 22.0), egui::TextEdit::singleline(&mut self.settings.workspace_dir));
                                if ui.button(RichText::new("Wybierz").size(10.0)).clicked() {
                                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                        self.settings.workspace_dir = folder.to_string_lossy().to_string();
                                    }
                                }
                            });

                            ui.add_space(6.0);

                            // 2. Visual Studio Code
                            ui.label(RichText::new("Tryb okien Visual Studio Code:").color(COLOR_TEXT_MUTED).size(10.5).strong());
                            ui.horizontal(|ui| {
                                let is_single = self.settings.vscode_mode == "single_window";
                                if ui.selectable_label(is_single, "Jedno okno").clicked() {
                                    self.settings.vscode_mode = "single_window".to_string();
                                }
                                if ui.selectable_label(!is_single, "Osobne okna").clicked() {
                                    self.settings.vscode_mode = "separate_windows".to_string();
                                }
                            });

                            ui.add_space(4.0);
                            ui.checkbox(&mut self.settings.open_pdf_in_vscode, "Dziel ekran: Kod po lewej, PDF po prawej");

                            ui.add_space(6.0);

                            // 3. Standard C++
                            ui.label(RichText::new("Standard C++ kompilacji zadań:").color(COLOR_TEXT_MUTED).size(10.5).strong());
                            ui.horizontal(|ui| {
                                for std in ["c++17", "c++20", "c++23"] {
                                    if ui.selectable_label(self.settings.cpp_standard == std, std.to_uppercase()).clicked() {
                                        self.settings.cpp_standard = std.to_string();
                                    }
                                }
                            });

                            ui.add_space(4.0);
                            ui.checkbox(&mut self.settings.sanitize_enabled, "Strict Debug: -fsanitize=address,undefined");

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(8.0);

                            // --- Sekcja 2: Integracje i Narzędzia ---
                            ui.label(RichText::new("Integracje i Narzędzia").color(COLOR_CYAN_ACCENT).size(11.5).strong());
                            ui.add_space(5.0);

                            ui.checkbox(&mut self.settings.notifications, "Powiadomienia pulpitu o testach");
                            ui.checkbox(&mut self.settings.auto_detect_downloads, "Automatycznie wykrywaj pobieranie PDF");

                            ui.add_space(6.0);

                            ui.label(RichText::new("Sprzęt: Stream Deck MK.2 USB HID (100% Rust Driver)").color(COLOR_CYAN_ACCENT).size(10.5));

                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(6.0);

                            // Wyłącz AlgoDeck
                            let btn_stop = ui.add_sized(
                                vec2(ui.available_width(), 28.0),
                                egui::Button::new(RichText::new("🛑 Wyłącz daemona AlgoDeck").color(Color32::from_rgb(248, 113, 113)).size(10.5))
                                    .fill(Color32::from_rgb(30, 20, 20)),
                            );
                            if btn_stop.clicked() {
                                crate::gui::tray::stop_algodeck();
                            }
                        });
                });
            });
        });
    }
}
