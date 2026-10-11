use egui::{Color32, Rounding, Stroke, Visuals};

pub const COLOR_BG_DARK: Color32 = Color32::from_rgb(8, 11, 17);       // #080b11
pub const COLOR_SURFACE: Color32 = Color32::from_rgb(17, 23, 38);      // #111726
pub const COLOR_SURFACE_LIGHT: Color32 = Color32::from_rgb(24, 34, 56); // #182238
pub const COLOR_BORDER: Color32 = Color32::from_rgb(30, 41, 59);        // #1e293b
pub const COLOR_CYAN: Color32 = Color32::from_rgb(2, 132, 199);         // #0284c7
pub const COLOR_CYAN_ACCENT: Color32 = Color32::from_rgb(56, 189, 248);  // #38bdf8
pub const COLOR_TEXT_MAIN: Color32 = Color32::from_rgb(248, 250, 252);  // #f8fafc
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(148, 163, 184); // #94a3b8
pub const COLOR_GREEN: Color32 = Color32::from_rgb(16, 185, 129);       // #10b981
pub const COLOR_RED: Color32 = Color32::from_rgb(239, 68, 68);          // #ef4444

pub fn configure_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(COLOR_TEXT_MAIN);
    visuals.panel_fill = COLOR_BG_DARK;
    visuals.window_fill = COLOR_BG_DARK;
    visuals.faint_bg_color = COLOR_SURFACE;
    visuals.extreme_bg_color = COLOR_SURFACE_LIGHT;

    visuals.widgets.noninteractive.bg_fill = COLOR_SURFACE;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0f32, COLOR_BORDER);
    visuals.widgets.noninteractive.rounding = Rounding::same(6.0);

    visuals.widgets.inactive.bg_fill = COLOR_SURFACE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0f32, COLOR_BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0f32, COLOR_TEXT_MUTED);
    visuals.widgets.inactive.rounding = Rounding::same(6.0);

    visuals.widgets.hovered.bg_fill = COLOR_SURFACE_LIGHT;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0f32, COLOR_CYAN_ACCENT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0f32, COLOR_TEXT_MAIN);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);

    visuals.widgets.active.bg_fill = COLOR_CYAN;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0f32, COLOR_CYAN_ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0f32, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(6.0);

    visuals.widgets.open.bg_fill = COLOR_SURFACE_LIGHT;
    visuals.widgets.open.rounding = Rounding::same(6.0);

    visuals.selection.bg_fill = COLOR_CYAN;
    visuals.selection.stroke = Stroke::new(1.0f32, COLOR_CYAN_ACCENT);

    ctx.set_visuals(visuals);
}
