use egui::{vec2, Align, Color32, Layout, RichText, Rounding, Stroke, Ui};
use std::path::{Path, PathBuf};

use crate::api::{ApiClient, ProblemInfo};
use crate::settings::AppSettings;
use crate::theme::*;

fn paint_dashed_rect(painter: &egui::Painter, rect: egui::Rect, rounding: f32, color: Color32, width: f32) {
    let r = rounding;
    let stroke = Stroke::new(width, color);
    let dash = 6.0;
    let gap = 5.0;

    // Top horizontal edge
    let mut x = rect.left() + r;
    while x < rect.right() - r {
        let x_end = (x + dash).min(rect.right() - r);
        painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x_end, rect.top())], stroke);
        x += dash + gap;
    }

    // Bottom horizontal edge
    let mut x = rect.left() + r;
    while x < rect.right() - r {
        let x_end = (x + dash).min(rect.right() - r);
        painter.line_segment([egui::pos2(x, rect.bottom()), egui::pos2(x_end, rect.bottom())], stroke);
        x += dash + gap;
    }

    // Left vertical edge
    let mut y = rect.top() + r;
    while y < rect.bottom() - r {
        let y_end = (y + dash).min(rect.bottom() - r);
        painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.left(), y_end)], stroke);
        y += dash + gap;
    }

    // Right vertical edge
    let mut y = rect.top() + r;
    while y < rect.bottom() - r {
        let y_end = (y + dash).min(rect.bottom() - r);
        painter.line_segment([egui::pos2(rect.right(), y), egui::pos2(rect.right(), y_end)], stroke);
        y += dash + gap;
    }
}

#[derive(PartialEq)]
pub enum ActiveTab {
    NewTask,
    TasksList,
    Settings,
}

#[derive(PartialEq)]
pub enum NewTaskMode {
    Pdf,
    Manual,
}

pub struct AlgoDeckApp {
    pub current_tab: ActiveTab,
    pub settings: AppSettings,

    // Tab 1: Nowe Zadanie
    pub new_mode: NewTaskMode,
    pub pdf_path: String,
    pub zip_path: String,
    pub task_code: String,
    pub task_title: String,
    pub time_limit: f64,
    pub memory_limit: u64,
    pub manual_cin: String,
    pub manual_cout: String,
    pub is_submitting: bool,

    // Tab 2: Zadania
    pub problems: Vec<ProblemInfo>,
    pub active_problem_id: String,
    pub selected_problem_idx: Option<usize>,
    pub filter_query: String,
    pub is_launcher_status: bool,

    // Komunikaty statusu (Toast)
    pub status_message: Option<(String, bool)>, // (treść, is_error)
}

impl AlgoDeckApp {
    pub fn new(initial_pdf: Option<String>) -> Self {
        let settings = AppSettings::load();
        let (problems, active_id) = ApiClient::fetch_problems(&settings.workspace_dir);

        let mut app = Self {
            current_tab: ActiveTab::NewTask,
            settings,
            new_mode: NewTaskMode::Pdf,
            pdf_path: String::new(),
            zip_path: String::new(),
            task_code: String::new(),
            task_title: String::new(),
            time_limit: 1.0,
            memory_limit: 128,
            manual_cin: String::new(),
            manual_cout: String::new(),
            is_submitting: false,
            problems,
            active_problem_id: active_id,
            selected_problem_idx: None,
            filter_query: String::new(),
            is_launcher_status: false,
            status_message: None,
        };

        if let Some(pdf) = initial_pdf {
            if Path::new(&pdf).exists() {
                app.set_pdf(pdf);
            }
        }

        app
    }

    pub fn set_pdf(&mut self, path: String) {
        self.new_mode = NewTaskMode::Pdf;
        self.pdf_path = path.clone();

        // Spróbuj auto-parsować metadane z pliku PDF
        if let Some(res) = ApiClient::auto_parse_pdf(&path) {
            if let Some(pid) = res.problem_id {
                if self.task_code.is_empty() {
                    self.task_code = pid;
                }
            }
            if let Some(title) = res.title {
                if self.task_title.is_empty() {
                    self.task_title = title;
                }
            }
            if let Some(t) = res.time_limit_sec {
                self.time_limit = t;
            }
            if let Some(m) = res.memory_limit_mb {
                self.memory_limit = m;
            }
            self.set_status("Sparsowano metadane z pliku PDF!", false);
        } else {
            let stem = Path::new(&path)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
                .to_lowercase();
            if self.task_code.is_empty() {
                self.task_code = stem.chars().take(5).collect();
            }
            self.set_status("Dołączono plik PDF zadania.", false);
        }
    }

    pub fn set_status(&mut self, msg: &str, is_error: bool) {
        self.status_message = Some((msg.to_string(), is_error));
    }

    pub fn refresh_problems(&mut self) {
        let (problems, active) = ApiClient::fetch_problems(&self.settings.workspace_dir);
        self.problems = problems;
        self.active_problem_id = active;
        self.selected_problem_idx = if !self.problems.is_empty() { Some(0) } else { None };
    }
}

impl eframe::App for AlgoDeckApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(1000));
        configure_theme(ctx);

        // Obsługa natywnego Drag-and-Drop plików z systemu
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path {
                        let path_str = path.to_string_lossy().to_string();
                        let lower = path_str.to_lowercase();
                        if lower.ends_with(".pdf") {
                            self.set_pdf(path_str);
                        } else if lower.ends_with(".zip") {
                            self.zip_path = path_str;
                            self.set_status("Dołączono paczkę testów .zip!", false);
                        }
                    }
                }
            }
        });

        // Obsługa trybu okienka statusu uruchomienia (z menu systemowego / wyszukiwarki)
        if self.is_launcher_status {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(COLOR_BG_DARK))
                .show(ctx, |ui| {
                    self.render_launcher_status_view(ctx, ui);
                });
            return;
        }

        // Główny kontener aplikacji
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(COLOR_BG_DARK))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                // 1. Customowy Pasek Tytułowy (BadWords-style: 34px, Flush, bez przechwytywania kursora)
                self.render_custom_titlebar(ctx, ui);

                // 2. Pasek Zakładek (Nowe Zadanie, Zadania, Ustawienia) - Czyste napisy bez kwadracików
                self.render_tab_bar(ui);

                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);

                // 3. Zawartość aktywnej zakładki
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                        match self.current_tab {
                            ActiveTab::NewTask => self.render_tab_new_task(ui),
                            ActiveTab::TasksList => self.render_tab_tasks(ui),
                            ActiveTab::Settings => self.render_tab_settings(ui),
                        }
                    });

                // 4. Dolny pasek statusu (Toast)
                if let Some((ref msg, is_err)) = self.status_message {
                    ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                        let fill = if is_err { Color32::from_rgb(127, 29, 29) } else { Color32::from_rgb(6, 78, 59) };
                        let stroke = if is_err { COLOR_RED } else { COLOR_GREEN };
                        egui::Frame::none()
                            .fill(fill)
                            .stroke(Stroke::new(1.0f32, stroke))
                            .rounding(Rounding::same(5.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new(msg).color(Color32::WHITE).size(11.0).strong());
                            });
                    });
                }
            });
    }
}

impl AlgoDeckApp {
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

                    // Przeciąganie okna bez przechwytywania kursora
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
                                .rounding(Rounding::same(5.0)),
                        );

                        if btn.clicked() {
                            self.current_tab = tab;
                            if self.current_tab == ActiveTab::TasksList {
                                self.refresh_problems();
                            }
                        }
                        ui.add_space(2.0);
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let is_alive = ApiClient::is_alive();
                        if is_alive {
                            let btn_shutdown = ui.add(
                                egui::Button::new(RichText::new("Wyłącz").color(Color32::from_rgb(254, 202, 202)).size(11.0).strong())
                                    .fill(Color32::from_rgb(127, 29, 29))
                                    .stroke(Stroke::NONE)
                                    .rounding(Rounding::same(5.0)),
                            );
                            if btn_shutdown.clicked() {
                                ApiClient::shutdown();
                                self.set_status("Wyłączono AlgoDeck. Profile Stream Decka przywrócone.", false);
                            }
                        } else {
                            let btn_start = ui.add(
                                egui::Button::new(RichText::new("Włącz").color(Color32::from_rgb(209, 250, 229)).size(11.0).strong())
                                    .fill(Color32::from_rgb(5, 150, 105))
                                    .stroke(Stroke::NONE)
                                    .rounding(Rounding::same(5.0)),
                            );
                            if btn_start.clicked() {
                                ApiClient::start_daemon();
                                self.set_status("Uruchamianie AlgoDeck w tle...", false);
                            }
                        }
                    });
                });
            });
    }

    // ----------------- Obszar Upuszczania Plików (Styl Rozszerzenia) -----------------
    fn render_drop_zone(
        &mut self,
        ui: &mut Ui,
        title: &str,
        hint_empty: &str,
        is_zip: bool,
    ) {
        let has_file = if is_zip { !self.zip_path.is_empty() } else { !self.pdf_path.is_empty() };
        let card_height = 58.0;
        let available_width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(vec2(available_width, card_height), egui::Sense::hover());
        let is_hovered = response.hovered();

        let bg_color = if has_file {
            Color32::from_rgb(15, 23, 42)
        } else if is_hovered {
            Color32::from_rgb(18, 28, 48)
        } else {
            Color32::from_rgb(11, 17, 30)
        };
        ui.painter().rect_filled(rect, Rounding::same(6.0), bg_color);

        let border_color = if has_file {
            COLOR_CYAN_ACCENT
        } else if is_hovered {
            Color32::from_rgb(56, 189, 248)
        } else {
            Color32::from_rgb(2, 132, 199)
        };
        paint_dashed_rect(ui.painter(), rect, 6.0, border_color, 1.5);

        let file_val = if is_zip { self.zip_path.clone() } else { self.pdf_path.clone() };
        let mut clear_clicked = false;
        let mut pick_clicked = false;

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect.shrink(6.0)), |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add_space(4.0);
                let icon_str = if is_zip { "📦" } else { "📄" };
                ui.label(RichText::new(icon_str).size(22.0));
                ui.add_space(8.0);

                ui.vertical(|ui| {
                    ui.label(RichText::new(title).color(Color32::WHITE).size(12.0).strong());
                    if has_file {
                        let filename = Path::new(&file_val)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| file_val.clone());
                        let short = if filename.len() > 30 {
                            format!("{}...", &filename[..27])
                        } else {
                            filename
                        };
                        ui.label(RichText::new(format!("✓ {}", short)).color(COLOR_CYAN_ACCENT).size(10.0).strong());
                    } else {
                        ui.label(RichText::new(hint_empty).color(COLOR_TEXT_MUTED).size(10.0));
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(6.0);
                    if has_file {
                        let btn_clear = ui.add(
                            egui::Button::new(RichText::new("✕ Usuń").color(Color32::from_rgb(248, 113, 113)).size(10.5).strong())
                                .fill(Color32::from_rgb(69, 10, 10))
                                .stroke(Stroke::new(1.0f32, Color32::from_rgb(185, 28, 28)))
                                .rounding(Rounding::same(5.0)),
                        );
                        if btn_clear.clicked() {
                            clear_clicked = true;
                        }
                    } else {
                        let btn_label = if is_zip { "Wybierz ZIP" } else { "Wybierz PDF" };
                        let btn_pick = ui.add(
                            egui::Button::new(RichText::new(btn_label).color(Color32::from_rgb(56, 189, 248)).size(11.0).strong())
                                .fill(Color32::from_rgb(14, 30, 52))
                                .stroke(Stroke::new(1.0f32, Color32::from_rgb(2, 132, 199)))
                                .rounding(Rounding::same(5.0)),
                        );
                        if btn_pick.clicked() {
                            pick_clicked = true;
                        }
                    }
                });
            });
        });

        if clear_clicked {
            if is_zip {
                self.zip_path.clear();
            } else {
                self.pdf_path.clear();
            }
        }
        if pick_clicked {
            if is_zip {
                if let Some(file) = rfd::FileDialog::new().add_filter("ZIP", &["zip"]).pick_file() {
                    self.zip_path = file.to_string_lossy().to_string();
                    self.set_status("Dołączono paczkę testów .zip!", false);
                }
            } else {
                if let Some(file) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file() {
                    let p = file.to_string_lossy().to_string();
                    self.set_pdf(p);
                }
            }
        }
    }

    fn render_parsed_pdf_card(&mut self, ui: &mut Ui) {
        let filename = Path::new(&self.pdf_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.pdf_path.clone());
        let short_name = if filename.len() > 28 {
            format!("{}...", &filename[..25])
        } else {
            filename
        };

        let mut clear_pdf = false;

        egui::Frame::none()
            .fill(Color32::from_rgb(14, 22, 38))
            .rounding(Rounding::same(6.0))
            .stroke(Stroke::new(1.2f32, COLOR_CYAN_ACCENT))
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📄").size(18.0));
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(short_name).color(Color32::WHITE).size(11.5).strong());
                        ui.label(RichText::new("✓ Metadane sparsowane z pliku PDF").color(COLOR_CYAN_ACCENT).size(9.5).strong());
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let btn_del = ui.add(
                            egui::Button::new(RichText::new("✕ Usuń").color(Color32::from_rgb(248, 113, 113)).size(10.5).strong())
                                .fill(Color32::from_rgb(69, 10, 10))
                                .stroke(Stroke::new(1.0f32, Color32::from_rgb(185, 28, 28)))
                                .rounding(Rounding::same(4.0)),
                        );
                        if btn_del.clicked() {
                            clear_pdf = true;
                        }
                    });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(5.0);

                // Dane sparsowane z PDF (Kod, Nazwa, Limity)
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Kod:").color(COLOR_TEXT_MUTED).size(10.5).strong());
                    let code_str = if self.task_code.is_empty() { "auto".to_string() } else { self.task_code.to_uppercase() };
                    ui.label(RichText::new(code_str).color(COLOR_CYAN_ACCENT).size(11.5).strong());

                    ui.add_space(10.0);
                    ui.label(RichText::new("Tytuł:").color(COLOR_TEXT_MUTED).size(10.5).strong());
                    let title_str = if self.task_title.is_empty() { self.task_code.to_uppercase() } else { self.task_title.clone() };
                    let title_disp = if title_str.len() > 16 { format!("{}...", &title_str[..14]) } else { title_str };
                    ui.label(RichText::new(title_disp).color(Color32::WHITE).size(11.0).strong());
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Limit czasu: {:.1}s", self.time_limit)).color(COLOR_TEXT_MUTED).size(10.5));
                    ui.add_space(10.0);
                    ui.label(RichText::new(format!("RAM: {} MB", self.memory_limit)).color(COLOR_TEXT_MUTED).size(10.5));
                });
            });

        if clear_pdf {
            self.pdf_path.clear();
            self.task_code.clear();
            self.task_title.clear();
        }
    }

    // ----------------- Zakładka 1: Nowe Zadanie (Styl Rozszerzenia) -----------------
    fn render_tab_new_task(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            // 1. Główny przycisk akcji przypięty do SAMEGO DOŁU okna
            ui.add_space(4.0);
            let btn_submit = ui.add_sized(
                vec2(ui.available_width(), 36.0),
                egui::Button::new(RichText::new("⚡ Utwórz Workspace i Otwórz VS Code").color(Color32::WHITE).size(12.0).strong())
                    .fill(COLOR_CYAN)
                    .stroke(Stroke::NONE)
                    .rounding(Rounding::same(6.0)),
            );

            // 2. Cała zawartość formularza wypełniająca przestrzeń od góry
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                // Przełącznik trybu (PDF vs Ręczne)
                ui.horizontal(|ui| {
                    let is_pdf = self.new_mode == NewTaskMode::Pdf;
                    let btn_pdf = ui.add(
                        egui::Button::new(RichText::new("📄 Plik PDF / ZIP").color(if is_pdf { Color32::WHITE } else { COLOR_TEXT_MUTED }).size(11.5).strong())
                            .fill(if is_pdf { COLOR_CYAN } else { Color32::from_rgb(16, 22, 34) })
                            .stroke(Stroke::NONE)
                            .rounding(Rounding::same(5.0)),
                    );
                    if btn_pdf.clicked() {
                        self.new_mode = NewTaskMode::Pdf;
                    }

                    let is_man = self.new_mode == NewTaskMode::Manual;
                    let btn_man = ui.add(
                        egui::Button::new(RichText::new("✏ Ręczne zadanie").color(if is_man { Color32::WHITE } else { COLOR_TEXT_MUTED }).size(11.5).strong())
                            .fill(if is_man { COLOR_CYAN } else { Color32::from_rgb(16, 22, 34) })
                            .stroke(Stroke::NONE)
                            .rounding(Rounding::same(5.0)),
                    );
                    if btn_man.clicked() {
                        self.new_mode = NewTaskMode::Manual;
                    }
                });

                ui.add_space(8.0);

                if self.new_mode == NewTaskMode::Pdf {
                    // W trybie PDF:
                    // Jeśli plik PDF jest dodany, pokazujemy w tym miejscu sparsowane dane zadania
                    if !self.pdf_path.is_empty() {
                        self.render_parsed_pdf_card(ui);
                    } else {
                        self.render_drop_zone(ui, "Treść zadania (.pdf)", "Przeciągnij plik PDF tutaj", false);
                    }

                    ui.add_space(8.0);

                    // Poniżej zawsze dostępny obszar na testy ZIP
                    self.render_drop_zone(ui, "Testy w archiwum (.zip)", "Przeciągnij archiwum ZIP tutaj", true);

                    ui.add_space(8.0);

                    // Karta informacyjna środowiska i opcje kompilacji wypełniająca przestrzeń
                    egui::Frame::none()
                        .fill(COLOR_SURFACE)
                        .rounding(Rounding::same(6.0))
                        .stroke(Stroke::new(1.0f32, COLOR_BORDER))
                        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("⚙ Opcje i Środowisko:").color(COLOR_CYAN_ACCENT).size(10.5).strong());
                            ui.add_space(3.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("C++:").color(COLOR_TEXT_MUTED).size(10.0).strong());
                                for std in ["c++17", "c++20", "c++23"] {
                                    if ui.selectable_label(self.settings.cpp_standard == std, std.to_uppercase()).clicked() {
                                        self.settings.cpp_standard = std.to_string();
                                    }
                                }
                            });
                            ui.add_space(2.0);
                            ui.checkbox(&mut self.settings.open_pdf_in_vscode, "Dziel ekran: Kod po lewej, PDF po prawej");
                            ui.add_space(3.0);
                            ui.label(RichText::new("• Rozpoznawanie nazwy i limitów z treści PDF").color(COLOR_TEXT_MUTED).size(9.5));
                            ui.label(RichText::new("• Automatyczny profil Stream Deck i testy").color(COLOR_TEXT_MUTED).size(9.5));
                        });
                } else {
                    // Tryb ręczny: Karta z polami formularza
                    egui::Frame::none()
                        .fill(COLOR_SURFACE)
                        .rounding(Rounding::same(6.0))
                        .stroke(Stroke::new(1.0f32, COLOR_BORDER))
                        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("Kod zadania * (np. chw, kol, sil):").color(COLOR_TEXT_MUTED).size(10.5).strong());
                            ui.add_sized(vec2(ui.available_width(), 24.0), egui::TextEdit::singleline(&mut self.task_code));

                            ui.add_space(5.0);
                            ui.label(RichText::new("Pełny tytuł (opcjonalny):").color(COLOR_TEXT_MUTED).size(10.5).strong());
                            ui.add_sized(vec2(ui.available_width(), 24.0), egui::TextEdit::singleline(&mut self.task_title));

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Czas (s):").color(COLOR_TEXT_MUTED).size(10.5).strong());
                                ui.add(egui::DragValue::new(&mut self.time_limit).speed(0.1).range(0.1..=10.0));
                                ui.add_space(8.0);
                                ui.label(RichText::new("RAM (MB):").color(COLOR_TEXT_MUTED).size(10.5).strong());
                                ui.add(egui::DragValue::new(&mut self.memory_limit).speed(16).range(16..=2048));
                            });

                            ui.add_space(6.0);
                            ui.label(RichText::new("Przykładowe wejście cin:").color(COLOR_TEXT_MUTED).size(10.5).strong());
                            ui.add_sized(vec2(ui.available_width(), 44.0), egui::TextEdit::multiline(&mut self.manual_cin));
                            ui.add_space(4.0);
                            ui.label(RichText::new("Oczekiwane wyjście cout:").color(COLOR_TEXT_MUTED).size(10.5).strong());
                            ui.add_sized(vec2(ui.available_width(), 44.0), egui::TextEdit::multiline(&mut self.manual_cout));
                        });
                }
            });

            // Obsługa kliknięcia przycisku
            if btn_submit.clicked() && !self.is_submitting {
                let code = self.task_code.trim().to_lowercase();
                if code.is_empty() {
                    self.set_status("Podaj kod lub nazwę zadania!", true);
                    return;
                }

                let title = if self.task_title.trim().is_empty() {
                    code.to_uppercase()
                } else {
                    self.task_title.trim().to_string()
                };

                self.is_submitting = true;
                let result = if self.new_mode == NewTaskMode::Pdf && !self.pdf_path.trim().is_empty() {
                    ApiClient::create_task_pdf(&self.pdf_path, &code, &title, self.time_limit, self.memory_limit).map(|_| ())
                } else {
                    ApiClient::create_task_manual(&code, &title, self.time_limit, self.memory_limit, &self.manual_cin, &self.manual_cout)
                };
                self.is_submitting = false;

                match result {
                    Ok(_) => {
                        self.set_status(&format!("Utworzono zadanie {}! Otwarto VS Code.", code.to_uppercase()), false);
                        ApiClient::open_in_vscode(&code, &self.settings.workspace_dir);
                        self.refresh_problems();
                        self.task_code.clear();
                        self.task_title.clear();
                        self.pdf_path.clear();
                        self.zip_path.clear();
                    }
                    Err(err) => {
                        self.set_status(&format!("Błąd tworzenia zadania: {}", err), true);
                    }
                }
            }
        });
    }

    // ----------------- Zakładka 2: Zarządzanie Zadaniami -----------------
    fn render_tab_tasks(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            // 1. Dolna karta z akcjami dla wybranego zadania (dokładna wysokość na samym dole)
            if let Some(idx) = self.selected_problem_idx {
                if idx < self.problems.len() {
                    let prob = self.problems[idx].clone();
                    ui.allocate_ui_with_layout(vec2(ui.available_width(), 96.0), Layout::top_down(Align::Min), |ui| {
                        egui::Frame::none()
                            .fill(COLOR_SURFACE)
                            .rounding(Rounding::same(6.0))
                            .stroke(Stroke::new(1.0f32, COLOR_BORDER))
                            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("{} [{}]", prob.title, prob.problem_id.to_uppercase())).color(COLOR_CYAN_ACCENT).size(12.0).strong());
                                ui.add_space(5.0);

                                // Szmaragdowo zielony przycisk aktywacji na pełną szerokość
                                let btn_activate = ui.add_sized(
                                    vec2(ui.available_width(), 32.0),
                                    egui::Button::new(RichText::new("⚡ Aktywuj (SD + VS Code)").color(Color32::WHITE).size(11.5).strong())
                                        .fill(Color32::from_rgb(16, 185, 129))
                                        .stroke(Stroke::NONE)
                                        .rounding(Rounding::same(5.0)),
                                );
                                if btn_activate.clicked() {
                                    let _ = ApiClient::set_active_problem(&prob.problem_id);
                                    ApiClient::open_in_vscode(&prob.problem_id, &self.settings.workspace_dir);
                                    self.active_problem_id = prob.problem_id.clone();
                                    self.set_status(&format!("Aktywowano zadanie: {}", prob.problem_id.to_uppercase()), false);
                                }

                                ui.add_space(4.0);

                                ui.horizontal(|ui| {
                                    if ui.button(RichText::new("📁 Folder").size(11.0)).clicked() {
                                        let wpath = PathBuf::from(&self.settings.workspace_dir).join(&prob.problem_id);
                                        let _ = open::that(&wpath);
                                    }

                                    if ui.button(RichText::new("📦 +Testy").size(11.0)).clicked() {
                                        if let Some(zip_file) = rfd::FileDialog::new().add_filter("Archiwum ZIP", &["zip"]).pick_file() {
                                            match ApiClient::add_tests_zip(&prob.problem_id, &zip_file) {
                                                Ok(count) => {
                                                    self.set_status(&format!("Dodano {} testów do zadania!", count), false);
                                                    self.refresh_problems();
                                                }
                                                Err(err) => {
                                                    self.set_status(&format!("Błąd wgrywania testów: {}", err), true);
                                                }
                                            }
                                        }
                                    }

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.add(egui::Button::new(RichText::new("🗑 Usuń").color(Color32::from_rgb(248, 113, 113)).size(11.0).strong()).fill(Color32::from_rgb(127, 29, 29))).clicked() {
                                            let _ = ApiClient::delete_problem(&prob.problem_id, &self.settings.workspace_dir);
                                            self.refresh_problems();
                                            self.set_status("Usunięto zadanie z workspace.", false);
                                        }
                                    });
                                });
                            });
                    });
                }
            }

            ui.add_space(4.0);

            // 2. Cała górna przestrzeń: Nagłówek, wyszukiwarka i lista zadań
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Zapisane zadania:").color(COLOR_TEXT_MAIN).size(12.0).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(RichText::new("Odśwież").size(11.0)).clicked() {
                            self.refresh_problems();
                        }
                    });
                });

                ui.add_space(4.0);

                // Pole wyszukiwania
                ui.add_sized(
                    vec2(ui.available_width(), 24.0),
                    egui::TextEdit::singleline(&mut self.filter_query).hint_text("Szukaj zadania..."),
                );

                ui.add_space(6.0);

                if self.selected_problem_idx.is_none() && !self.problems.is_empty() {
                    let active_idx = self.problems.iter().position(|p| p.problem_id == self.active_problem_id).unwrap_or(0);
                    self.selected_problem_idx = Some(active_idx);
                }

                let filtered_indices: Vec<usize> = self.problems
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| {
                        if self.filter_query.is_empty() {
                            true
                        } else {
                            let q = self.filter_query.to_lowercase();
                            p.problem_id.to_lowercase().contains(&q) || p.title.to_lowercase().contains(&q)
                        }
                    })
                    .map(|(i, _)| i)
                    .collect();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if filtered_indices.is_empty() {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("Brak zadań w wybranym workspace.").color(COLOR_TEXT_MUTED).size(11.5));
                        });
                    } else {
                        for &idx in &filtered_indices {
                            let prob = &self.problems[idx];
                            let is_active_sd = prob.problem_id == self.active_problem_id;
                            let is_selected = self.selected_problem_idx == Some(idx);

                            let bg = if is_selected { COLOR_SURFACE_LIGHT } else { COLOR_SURFACE };
                            let stroke = if is_active_sd {
                                Stroke::new(1.5f32, COLOR_GREEN)
                            } else if is_selected {
                                Stroke::new(1.0f32, COLOR_CYAN)
                            } else {
                                Stroke::new(1.0f32, COLOR_BORDER)
                            };

                            let resp = egui::Frame::none()
                                .fill(bg)
                                .stroke(stroke)
                                .rounding(Rounding::same(6.0))
                                .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new(&prob.title).color(COLOR_TEXT_MAIN).size(12.0).strong());
                                                ui.label(RichText::new(format!("[{}]", prob.problem_id.to_uppercase())).color(COLOR_CYAN_ACCENT).size(11.0).strong());
                                            });
                                            ui.label(RichText::new(format!("Testy: {}", prob.tests_count)).color(COLOR_TEXT_MUTED).size(10.0));
                                        });

                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if is_active_sd {
                                                ui.label(RichText::new("AKTYWNE").color(COLOR_GREEN).size(10.0).strong());
                                            }
                                        });
                                    });
                                });

                            if resp.response.interact(egui::Sense::click()).clicked() {
                                self.selected_problem_idx = Some(idx);
                            }
                            ui.add_space(4.0);
                        }
                    }
                });
            });
        });
    }

    // ----------------- Zakładka 3: Ustawienia (Jeden spójny obszar tła na pełną szerokość) -----------------
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

                            let platform_detected = if cfg!(target_os = "macos") {
                                "macOS (Oficjalny Stream Deck)"
                            } else if cfg!(target_os = "windows") {
                                "Windows (Oficjalny Stream Deck)"
                            } else {
                                "Linux (StreamController - automatycznie)"
                            };
                            ui.label(RichText::new(format!("Platforma: {}", platform_detected)).color(COLOR_CYAN_ACCENT).size(10.5));

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Serial SD:").color(COLOR_TEXT_MUTED).size(10.0));
                                ui.add_sized(vec2(ui.available_width(), 20.0), egui::TextEdit::singleline(&mut self.settings.streamdeck_serial));
                            });

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.button(RichText::new("Sync SD").size(10.5)).clicked() {
                                    ApiClient::sync_streamdeck();
                                    self.set_status("Profile Stream Decka zsynchronizowane!", false);
                                }
                                if ui.button(RichText::new("Ekran Zachęty").size(10.5)).clicked() {
                                    ApiClient::show_idle_screen();
                                    self.set_status("Przełączono Stream Deck na ekran zachęty.", false);
                                }
                            });

                            ui.add_space(6.0);

                            // Status serwera
                            let is_alive = ApiClient::is_alive();
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), egui::Sense::hover());
                                let dot_color = if is_alive { COLOR_GREEN } else { COLOR_RED };
                                ui.painter().circle_filled(rect.center(), 4.0, dot_color);
                                let status_lbl = if is_alive { "Serwer AlgoDeck: ONLINE (8080)" } else { "Serwer AlgoDeck: OFFLINE" };
                                ui.label(RichText::new(status_lbl).color(COLOR_TEXT_MAIN).size(10.5).strong());
                            });
                        });
                });
            });

            if btn_save.clicked() {
                match self.settings.save() {
                    Ok(_) => {
                        ApiClient::update_backend_settings(&self.settings);
                        self.set_status("Ustawienia zostały pomyślnie zapisane!", false);
                    }
                    Err(e) => self.set_status(&format!("Błąd zapisu: {}", e), true),
                }
            }
        });
    }

    fn render_launcher_status_view(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        let width = ui.available_width();

        // Górny pasek z przyciskiem zamknięcia
        egui::Frame::none()
            .fill(Color32::from_rgb(20, 24, 34))
            .show(ui, |ui| {
                ui.set_min_size(vec2(width, 32.0));
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(RichText::new("AlgoDeck ⚡").color(COLOR_CYAN_ACCENT).size(12.0).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(RichText::new("✕").color(COLOR_TEXT_MUTED).size(12.0)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
            });

        ui.add_space(12.0);

        egui::Frame::none()
            .fill(COLOR_SURFACE)
            .rounding(Rounding::same(8.0))
            .stroke(Stroke::new(1.0f32, COLOR_BORDER))
            .inner_margin(egui::Margin::symmetric(16.0, 14.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("⚡ AlgoDeck jest aktywny!").color(COLOR_CYAN_ACCENT).size(16.0).strong());
                    ui.add_space(6.0);
                    ui.label(RichText::new("Środowisko olimpijskie zostało uruchomione w tle.").color(COLOR_TEXT_MAIN).size(12.0));
                    ui.label(RichText::new("Ekran zachęty czeka na Stream Decku, a aplikacja czuwa w zasobniku systemowym.").color(COLOR_TEXT_MUTED).size(11.0));
                });

                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    let btn_gui = ui.add_sized(
                        vec2(ui.available_width() * 0.55, 34.0),
                        egui::Button::new(RichText::new("🖥 Otwórz Centrum Kontroli").color(Color32::WHITE).size(11.5).strong())
                            .fill(COLOR_CYAN)
                            .rounding(Rounding::same(6.0)),
                    );
                    if btn_gui.clicked() {
                        self.is_launcher_status = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(340.0, 580.0)));
                    }

                    let btn_min = ui.add_sized(
                        vec2(ui.available_width(), 34.0),
                        egui::Button::new(RichText::new("✕ Minimalizuj").color(COLOR_TEXT_MAIN).size(11.5).strong())
                            .fill(COLOR_SURFACE_LIGHT)
                            .stroke(Stroke::new(1.0f32, COLOR_BORDER))
                            .rounding(Rounding::same(6.0)),
                    );
                    if btn_min.clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
    }
}
