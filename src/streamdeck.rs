use image::{imageops, DynamicImage, Rgb, RgbImage, RgbaImage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use streamdeck::StreamDeck;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamDeckKey {
    pub id: usize,
    pub icon_name: String,
    pub action: String,
    pub status: String, // "IDLE", "RUNNING", "OK", "WA", "CE"
    pub custom_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenMode {
    Idle,
    Task(String), // problem_id
    Menu(usize),  // page_index
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDeckState {
    pub keys: Vec<StreamDeckKey>,
    pub active_problem_id: Option<String>,
    pub is_hardware_connected: bool,
}

pub struct StreamDeckEngine {
    state: Arc<Mutex<StreamDeckState>>,
    mode: Arc<Mutex<ScreenMode>>,
    running: Arc<AtomicBool>,
    dirty: Arc<AtomicBool>,
    brightness: Arc<std::sync::atomic::AtomicU8>,
    icon_cache: Arc<Mutex<HashMap<String, DynamicImage>>>,
}

impl StreamDeckEngine {
    pub fn new() -> Self {
        let mut keys = Vec::with_capacity(15);
        for i in 0..15 {
            keys.push(StreamDeckKey {
                id: i,
                icon_name: String::new(),
                action: String::new(),
                status: "IDLE".to_string(),
                custom_label: None,
            });
        }

        let initial_state = StreamDeckState {
            keys,
            active_problem_id: None,
            is_hardware_connected: false,
        };

        let engine = Self {
            state: Arc::new(Mutex::new(initial_state)),
            mode: Arc::new(Mutex::new(ScreenMode::Idle)),
            running: Arc::new(AtomicBool::new(true)),
            dirty: Arc::new(AtomicBool::new(true)),
            brightness: Arc::new(std::sync::atomic::AtomicU8::new(85)),
            icon_cache: Arc::new(Mutex::new(HashMap::new())),
        };

        engine.set_idle_screen();
        engine
    }

    pub fn set_brightness(&self, val: u8) {
        let clamped = val.clamp(5, 100);
        self.brightness.store(clamped, Ordering::SeqCst);
        self.dirty.store(true, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn get_brightness(&self) -> u8 {
        self.brightness.load(Ordering::Relaxed)
    }

    pub fn get_state(&self) -> StreamDeckState {
        self.state.lock().unwrap().clone()
    }

    fn find_icon_file(name: &str) -> Option<PathBuf> {
        let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf()));
        let env_icons = std::env::var("ALGODECK_ICONS_DIR").ok().map(PathBuf::from);

        let mut candidates = Vec::new();

        if let Some(ref env_dir) = env_icons {
            candidates.push(env_dir.join(name));
        }

        if let Some(ref exe) = exe_dir {
            candidates.push(exe.join("icons").join(name));
            candidates.push(exe.join("../icons").join(name));
            candidates.push(exe.join("frontend/icons").join(name));
            candidates.push(exe.join("../frontend/icons").join(name));
            candidates.push(exe.join("../../frontend/icons").join(name));
        }

        // Ścieżki systemowe per OS
        #[cfg(target_os = "windows")]
        {
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                candidates.push(PathBuf::from(local).join("AlgoDeck/icons").join(name));
            }
            if let Ok(appdata) = std::env::var("APPDATA") {
                candidates.push(PathBuf::from(appdata).join("AlgoDeck/icons").join(name));
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Some(home) = dirs::home_dir() {
                candidates.push(home.join(".local/share/algodeck/icons").join(name));
                candidates.push(home.join(".local/share/algodeck/frontend/icons").join(name));
            }
            candidates.push(PathBuf::from("/usr/local/share/algodeck/icons").join(name));
            candidates.push(PathBuf::from("/usr/share/algodeck/icons").join(name));
        }

        if let Some(data) = dirs::data_dir() {
            candidates.push(data.join("algodeck/icons").join(name));
        }

        // Ścieżki względne z bieżącego katalogu roboczego
        candidates.push(PathBuf::from("frontend/icons").join(name));
        candidates.push(PathBuf::from("icons").join(name));

        for c in candidates {
            if c.exists() {
                return Some(c);
            }
        }
        None
    }

    fn load_icon(&self, name: &str) -> DynamicImage {
        if name.is_empty() {
            return DynamicImage::ImageRgb8(RgbImage::from_pixel(72, 72, Rgb([0, 0, 0])));
        }

        let mut cache = self.icon_cache.lock().unwrap();
        if let Some(cached) = cache.get(name) {
            return cached.clone();
        }

        let img = if let Some(p) = Self::find_icon_file(name) {
            if let Ok(loaded) = image::open(&p) {
                let resized = loaded.resize_exact(72, 72, imageops::FilterType::Lanczos3);
                // Composite over pitch black #000000 background
                let mut base = RgbImage::from_pixel(72, 72, Rgb([0, 0, 0]));
                let rgba = resized.to_rgba8();
                for (x, y, pixel) in rgba.enumerate_pixels() {
                    let alpha = pixel[3] as f32 / 255.0;
                    if alpha > 0.0 {
                        let r = (pixel[0] as f32 * alpha) as u8;
                        let g = (pixel[1] as f32 * alpha) as u8;
                        let b = (pixel[2] as f32 * alpha) as u8;
                        base.put_pixel(x, y, Rgb([r, g, b]));
                    }
                }
                DynamicImage::ImageRgb8(base)
            } else {
                DynamicImage::ImageRgb8(RgbImage::from_pixel(72, 72, Rgb([0, 0, 0])))
            }
        } else {
            DynamicImage::ImageRgb8(RgbImage::from_pixel(72, 72, Rgb([0, 0, 0])))
        };

        cache.insert(name.to_string(), img.clone());
        img
    }

    // ----------------- 1. EKRAN POWITALNY: ALGO_IDLE -----------------
    pub fn set_idle_screen(&self) {
        let mut state = self.state.lock().unwrap();
        state.active_problem_id = None;
        *self.mode.lock().unwrap() = ScreenMode::Idle;

        for i in 0..15 {
            state.keys[i] = StreamDeckKey {
                id: i,
                icon_name: String::new(),
                action: String::new(),
                status: "IDLE".to_string(),
                custom_label: None,
            };
        }

        // Rząd 1 (Środek): [ ALGO ] [ ⚡ ] [ DECK ] (Klawisze 6, 7, 8)
        state.keys[6] = StreamDeckKey {
            id: 6,
            icon_name: "idle_logo_algo.png".to_string(),
            action: "start_or_resume".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };
        state.keys[7] = StreamDeckKey {
            id: 7,
            icon_name: "idle_logo_bolt.png".to_string(),
            action: "start_or_resume".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };
        state.keys[8] = StreamDeckKey {
            id: 8,
            icon_name: "idle_logo_deck.png".to_string(),
            action: "start_or_resume".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };

        self.dirty.store(true, Ordering::SeqCst);
    }

    // ----------------- 2. EKRAN ZADANIA: ALGO_TASK -----------------
    pub fn set_active_problem_with_meta(&self, problem_id: &str, current_idx: usize, total_count: usize) {
        let pid = problem_id.to_lowercase();
        let upper_pid = pid.to_uppercase();
        let _ = std::fs::write(std::env::temp_dir().join("algodeck_active_task.txt"), &upper_pid);
        #[cfg(unix)]
        {
            let _ = std::fs::write("/tmp/algodeck_active_task.txt", &upper_pid);
        }

        let mut state = self.state.lock().unwrap();
        state.active_problem_id = Some(pid.clone());
        *self.mode.lock().unwrap() = ScreenMode::Task(pid.clone());

        for i in 0..15 {
            state.keys[i] = StreamDeckKey {
                id: i,
                icon_name: String::new(),
                action: String::new(),
                status: "IDLE".to_string(),
                custom_label: None,
            };
        }

        // --- RZĄD 0 (GÓRA): [ = ] [ L1 ] [ L2 ] [ L3 ] [ = ] ---
        // Kliknięcie DOWOLNEGO klawisza w rzędzie 0 otwiera zadanie w VS Code!
        let chars: Vec<char> = pid.chars().collect();
        let c1 = chars.get(0).copied().unwrap_or('-');
        let c2 = chars.get(1).copied().unwrap_or('-');
        let c3 = chars.get(2).copied().unwrap_or('-');

        state.keys[0] = StreamDeckKey {
            id: 0,
            icon_name: "icon_equals.png".to_string(),
            action: "open_vscode".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };
        state.keys[1] = StreamDeckKey {
            id: 1,
            icon_name: format!("letter_{}.png", c1),
            action: "open_vscode".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };
        state.keys[2] = StreamDeckKey {
            id: 2,
            icon_name: format!("letter_{}.png", c2),
            action: "open_vscode".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };
        state.keys[3] = StreamDeckKey {
            id: 3,
            icon_name: format!("letter_{}.png", c3),
            action: "open_vscode".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };
        state.keys[4] = StreamDeckKey {
            id: 4,
            icon_name: "icon_equals.png".to_string(),
            action: "open_vscode".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };

        // --- RZĄD 1 (ŚRODEK): [ + ] [ ✓ ] [ ▶ ] [ ⏹ ] [ ⊞ ] ---
        state.keys[5] = StreamDeckKey {
            id: 5,
            icon_name: "icon_plus.png".to_string(),
            action: "new_task".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };
        state.keys[6] = StreamDeckKey {
            id: 6,
            icon_name: "icon_test_sec.png".to_string(),
            action: "run_all".to_string(),
            status: "OK".to_string(),
            custom_label: None,
        };
        state.keys[7] = StreamDeckKey {
            id: 7,
            icon_name: "icon_play_sec.png".to_string(),
            action: "run_interactive".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };
        state.keys[8] = StreamDeckKey {
            id: 8,
            icon_name: "icon_kill_sec.png".to_string(),
            action: "kill_process".to_string(),
            status: "DANGER".to_string(),
            custom_label: None,
        };
        state.keys[9] = StreamDeckKey {
            id: 9,
            icon_name: "icon_task_sec.png".to_string(),
            action: "open_menu".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };

        // --- RZĄD 2 (DÓŁ): [ ⚙ ] [ ↩ ] [ ‹ ] [Nr ] [ › ] ---
        state.keys[10] = StreamDeckKey {
            id: 10,
            icon_name: "icon_settings.png".to_string(),
            action: "open_settings".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };
        // Klawisz 11: Puste pole (czysta czerń OLED #000000)
        state.keys[11] = StreamDeckKey {
            id: 11,
            icon_name: String::new(),
            action: String::new(),
            status: "IDLE".to_string(),
            custom_label: None,
        };

        let left_icon = if total_count <= 1 { "icon_arrow_left_disabled.png" } else { "icon_arrow_left.png" };
        let right_icon = if total_count <= 1 { "icon_arrow_right_disabled.png" } else { "icon_arrow_right.png" };

        state.keys[12] = StreamDeckKey {
            id: 12,
            icon_name: left_icon.to_string(),
            action: "prev_task".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };

        // Kafelek numeru zadania: sprawdź czy plik task_num_X_Y.png istnieje
        let num_filename = format!("task_num_{}_{}.png", current_idx, total_count);
        state.keys[13] = StreamDeckKey {
            id: 13,
            icon_name: num_filename,
            action: "open_vscode".to_string(),
            status: "IDLE".to_string(),
            custom_label: Some(format!("{}/{}", current_idx, total_count)),
        };

        state.keys[14] = StreamDeckKey {
            id: 14,
            icon_name: right_icon.to_string(),
            action: "next_task".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };

        self.dirty.store(true, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn set_active_problem(&self, problem_id: &str) {
        self.set_active_problem_with_meta(problem_id, 1, 1);
    }

    pub fn get_active_problem_id(&self) -> Option<String> {
        let state = self.state.lock().unwrap();
        state.active_problem_id.clone()
    }

    // ----------------- 3. EKRAN GALERII ZADAŃ: ALGO_MENU -----------------
    pub fn set_menu_screen(&self, problems: &[String], page: usize) {
        let mut state = self.state.lock().unwrap();
        *self.mode.lock().unwrap() = ScreenMode::Menu(page);

        for i in 0..15 {
            state.keys[i] = StreamDeckKey {
                id: i,
                icon_name: String::new(),
                action: String::new(),
                status: "IDLE".to_string(),
                custom_label: None,
            };
        }

        let chunk_size = 10;
        let start_idx = page * chunk_size;
        let page_problems = problems.iter().skip(start_idx).take(chunk_size);

        for (idx, p_id) in page_problems.enumerate() {
            state.keys[idx] = StreamDeckKey {
                id: idx,
                icon_name: String::new(),
                action: format!("select_task:{}", p_id),
                status: if state.active_problem_id.as_deref() == Some(p_id) { "OK".to_string() } else { "IDLE".to_string() },
                custom_label: Some(p_id.to_uppercase()),
            };
        }

        // Klawisz 10: Strzałka powrotu ←
        state.keys[10] = StreamDeckKey {
            id: 10,
            icon_name: "icon_arrow_back.png".to_string(),
            action: "back_to_task".to_string(),
            status: "IDLE".to_string(),
            custom_label: None,
        };

        // Klawisz 14: Strzałka dalej › jeśli jest więcej stron
        if problems.len() > start_idx + chunk_size {
            state.keys[14] = StreamDeckKey {
                id: 14,
                icon_name: "icon_arrow_right.png".to_string(),
                action: format!("menu_page:{}", page + 1),
                status: "IDLE".to_string(),
                custom_label: None,
            };
        }

        self.dirty.store(true, Ordering::SeqCst);
    }

    pub fn update_key_status(&self, key_id: usize, status: &str, _subtext: Option<&str>) {
        let mut state = self.state.lock().unwrap();
        if key_id < 15 {
            state.keys[key_id].status = status.to_string();
            self.dirty.store(true, Ordering::SeqCst);
        }
    }

    // ----------------- RENDEROWANIE PRZYCISKU (NA PURE BLACK #000000) -----------------
    pub fn render_key_image(&self, key: &StreamDeckKey) -> DynamicImage {
        // 1. Jeśli mamy bezpośredni plik ikony na dysku, wczytaj go
        if !key.icon_name.is_empty() {
            let img = self.load_icon(&key.icon_name);
            // Jeśli ikona istnieje i nie jest pustym czarnym kwadratem, zwróć ją
            if Self::is_non_black(&img) {
                return img;
            }
        }

        // 2. Jeśli klawisz ma custom_label (np. w menu zadań lub numerze 1/4)
        if let Some(ref label) = key.custom_label {
            return Self::render_label_button(label, &key.status);
        }

        // 3. Proceduralny generator awaryjny (fallback) na tle OLED black jeśli plik PNG nie istnieje
        if !key.icon_name.is_empty() || !key.action.is_empty() {
            return Self::render_fallback_tile(&key.icon_name, &key.action, &key.status);
        }

        // 4. Domyślny całkowicie czarny ekran OLED (#000000)
        DynamicImage::ImageRgb8(RgbImage::from_pixel(72, 72, Rgb([0, 0, 0])))
    }

    pub fn render_fallback_tile(icon_name: &str, action: &str, status: &str) -> DynamicImage {
        let mut base = RgbImage::from_pixel(72, 72, Rgb([0, 0, 0]));

        // Litera zadania np. "letter_s.png"
        if icon_name.starts_with("letter_") {
            let char_part = icon_name.trim_start_matches("letter_").trim_end_matches(".png");
            let c = char_part.chars().next().unwrap_or('-').to_ascii_uppercase();
            let start_x = (72 - 5 * 3) / 2;
            let start_y = (72 - 7 * 3) / 2;
            Self::draw_scaled_text(&mut base, &c.to_string(), start_x, start_y, 3, Rgb([56, 189, 248]));
            return DynamicImage::ImageRgb8(base);
        }

        // Znak "="
        if icon_name == "icon_equals.png" {
            for y in [28, 29, 30, 42, 43, 44] {
                for x in 18..54 {
                    base.put_pixel(x, y, Rgb([56, 189, 248]));
                }
            }
            return DynamicImage::ImageRgb8(base);
        }

        // Ekran powitalny: ALGO / BOLT / DECK
        if icon_name == "idle_logo_algo.png" {
            return Self::render_badge_tile("ALGO", Rgb([56, 189, 248]), Rgb([248, 250, 252]));
        }
        if icon_name == "idle_logo_bolt.png" {
            Self::draw_bolt(&mut base, Rgb([250, 204, 21]));
            return DynamicImage::ImageRgb8(base);
        }
        if icon_name == "idle_logo_deck.png" {
            return Self::render_badge_tile("DECK", Rgb([56, 189, 248]), Rgb([248, 250, 252]));
        }

        // Przycisk "+" (nowe zadanie)
        if icon_name == "icon_plus.png" || action == "new_task" {
            for y in 33..39 {
                for x in 18..54 {
                    base.put_pixel(x, y, Rgb([16, 185, 129]));
                }
            }
            for x in 33..39 {
                for y in 18..54 {
                    base.put_pixel(x, y, Rgb([16, 185, 129]));
                }
            }
            for x in 10..62 {
                base.put_pixel(x, 10, Rgb([5, 150, 105]));
                base.put_pixel(x, 61, Rgb([5, 150, 105]));
            }
            for y in 10..62 {
                base.put_pixel(10, y, Rgb([5, 150, 105]));
                base.put_pixel(61, y, Rgb([5, 150, 105]));
            }
            return DynamicImage::ImageRgb8(base);
        }

        // Testy: run_all
        if icon_name == "icon_test_sec.png" || action == "run_all" {
            let (col, txt) = match status {
                "OK" => (Rgb([34, 197, 94]), "OK"),
                "WA" => (Rgb([239, 68, 68]), "WA"),
                "RUNNING" => (Rgb([234, 179, 8]), "RUN"),
                _ => (Rgb([56, 189, 248]), "TEST"),
            };
            return Self::render_badge_tile(txt, col, Rgb([255, 255, 255]));
        }

        // Interaktywne uruchomienie: run_interactive
        if icon_name == "icon_play_sec.png" || action == "run_interactive" {
            for x in 26..48 {
                let half_h = (x - 26) * 4 / 5;
                for y in (36 - half_h)..=(36 + half_h) {
                    base.put_pixel(x as u32, y as u32, Rgb([96, 165, 250]));
                }
            }
            return DynamicImage::ImageRgb8(base);
        }

        // Kill process
        if icon_name == "icon_kill_sec.png" || action == "kill_process" {
            for y in 24..48 {
                for x in 24..48 {
                    base.put_pixel(x, y, Rgb([239, 68, 68]));
                }
            }
            return DynamicImage::ImageRgb8(base);
        }

        // Menu zadań
        if icon_name == "icon_task_sec.png" || action == "open_menu" {
            for (bx, by) in [(18, 18), (38, 18), (18, 38), (38, 38)] {
                for y in by..(by + 16) {
                    for x in bx..(bx + 16) {
                        base.put_pixel(x, y, Rgb([56, 189, 248]));
                    }
                }
            }
            return DynamicImage::ImageRgb8(base);
        }

        // Ustawienia
        if icon_name == "icon_settings.png" || action == "open_settings" {
            return Self::render_badge_tile("SET", Rgb([148, 163, 184]), Rgb([248, 250, 252]));
        }

        // Strzałki nawigacji
        if icon_name.contains("arrow_left") || action == "prev_task" || action == "back_to_task" {
            for y in 18..54 {
                let dist = (y as i32 - 36).abs();
                let start_x = 24 + dist;
                for x in start_x..=(start_x + 5) {
                    if x < 54 {
                        base.put_pixel(x as u32, y as u32, Rgb([148, 163, 184]));
                    }
                }
            }
            return DynamicImage::ImageRgb8(base);
        }

        if icon_name.contains("arrow_right") || action == "next_task" {
            for y in 18..54 {
                let dist = (y as i32 - 36).abs();
                let end_x = 48 - dist;
                for x in (end_x - 5)..=end_x {
                    if x >= 18 {
                        base.put_pixel(x as u32, y as u32, Rgb([148, 163, 184]));
                    }
                }
            }
            return DynamicImage::ImageRgb8(base);
        }

        DynamicImage::ImageRgb8(base)
    }

    fn draw_bolt(img: &mut RgbImage, color: Rgb<u8>) {
        // Precyzyjny kształt błyskawicy ⚡
        let points = [
            (40, 14), (39, 15), (38, 16), (37, 17), (36, 18), (35, 19), (34, 20),
            (33, 21), (32, 22), (31, 23), (30, 24), (29, 25), (28, 26), (27, 27),
            (26, 28), (25, 29), (24, 30), (23, 31), (22, 32), (21, 33), (20, 34),
            (21, 35), (22, 35), (23, 35), (24, 35), (25, 35), (26, 35), (27, 35),
            (28, 35), (29, 35), (30, 35), (31, 35), (32, 35), (33, 35), (34, 35),
            (35, 35), (36, 35), (37, 35), (38, 35), (39, 35), (40, 35), (41, 35),
            (36, 36), (35, 37), (34, 38), (33, 39), (32, 40), (31, 41), (30, 42),
            (29, 43), (28, 44), (27, 45), (26, 46), (25, 47), (24, 48), (23, 49),
            (22, 50), (21, 51), (20, 52), (19, 53), (18, 54), (17, 55), (16, 56),
        ];
        for &(px, py) in &points {
            for dx in 0..3 {
                for dy in 0..2 {
                    let nx = px + dx + 10;
                    let ny = py + dy;
                    if nx < 72 && ny < 72 {
                        img.put_pixel(nx, ny, color);
                    }
                }
            }
        }
    }

    fn render_badge_tile(text: &str, border_color: Rgb<u8>, text_color: Rgb<u8>) -> DynamicImage {
        let mut base = RgbImage::from_pixel(72, 72, Rgb([0, 0, 0]));
        for x in 8..64 {
            base.put_pixel(x, 10, border_color);
            base.put_pixel(x, 61, border_color);
        }
        for y in 14..58 {
            base.put_pixel(6, y, border_color);
            base.put_pixel(65, y, border_color);
        }
        let len = text.chars().count() as i32;
        let scale = if len <= 4 { 2 } else { 1 };
        let char_w = 5 * scale + scale;
        let total_w = len * char_w - scale;
        let start_x = ((72 - total_w) / 2).max(8);
        let start_y = (72 - 7 * scale) / 2;
        Self::draw_scaled_text(&mut base, text, start_x, start_y, scale, text_color);
        DynamicImage::ImageRgb8(base)
    }

    fn is_non_black(img: &DynamicImage) -> bool {
        let rgb = img.to_rgb8();
        for p in rgb.pixels() {
            if p[0] > 5 || p[1] > 5 || p[2] > 5 {
                return true;
            }
        }
        false
    }

    fn render_composed_indicator(label: &str) -> Option<DynamicImage> {
        let mut glyphs: Vec<RgbaImage> = Vec::new();
        for c in label.chars() {
            let icon_file = if c == '/' {
                "letter_slash.png".to_string()
            } else if c.is_ascii_digit() {
                format!("letter_{}.png", c)
            } else {
                format!("letter_{}.png", c.to_ascii_lowercase())
            };

            let path = Self::find_icon_file(&icon_file)?;
            let img = image::open(&path).ok()?.to_rgba8();

            // Oblicz bounding box (nieczarne piksele litery/cyfry)
            let mut min_x = img.width();
            let mut max_x = 0;
            let mut min_y = img.height();
            let mut max_y = 0;
            let mut has_pixel = false;

            for y in 0..img.height() {
                for x in 0..img.width() {
                    let p = img.get_pixel(x, y);
                    if p[0] > 15 || p[1] > 15 || p[2] > 15 {
                        has_pixel = true;
                        if x < min_x { min_x = x; }
                        if x > max_x { max_x = x; }
                        if y < min_y { min_y = y; }
                        if y > max_y { max_y = y; }
                    }
                }
            }

            if has_pixel && max_x >= min_x && max_y >= min_y {
                let crop_w = max_x - min_x + 1;
                let crop_h = max_y - min_y + 1;
                let cropped = imageops::crop_imm(&img, min_x, min_y, crop_w, crop_h).to_image();
                glyphs.push(cropped);
            } else {
                glyphs.push(img);
            }
        }

        if glyphs.is_empty() {
            return None;
        }

        // Bazowa wysokość tekstu na kafelku 72x72: 14px (zgodnie z oryginalnym projektem graficznym i odstępami od boków)
        let base_h: f32 = 14.0;
        let spacing: i32 = 2;
        let mut total_raw_w: f32 = 0.0;
        for g in &glyphs {
            let aspect = g.width() as f32 / g.height().max(1) as f32;
            total_raw_w += aspect * base_h;
        }
        total_raw_w += (spacing * (glyphs.len() as i32 - 1).max(0)) as f32;

        let max_w: f32 = 44.0;
        let scale = if total_raw_w > max_w { max_w / total_raw_w } else { 1.0 };
        let actual_h = (base_h * scale).round().max(10.0) as u32;

        let mut scaled_glyphs: Vec<RgbaImage> = Vec::new();
        let mut total_w: i32 = 0;
        for g in glyphs {
            let aspect = g.width() as f32 / g.height().max(1) as f32;
            let new_w = (actual_h as f32 * aspect).round().max(1.0) as u32;
            let scaled = imageops::resize(&g, new_w, actual_h, imageops::FilterType::Lanczos3);
            total_w += new_w as i32;
            scaled_glyphs.push(scaled);
        }
        total_w += spacing * (scaled_glyphs.len() as i32 - 1).max(0);

        let mut base = RgbImage::from_pixel(72, 72, Rgb([0, 0, 0]));
        let start_x = ((72 - total_w) / 2).max(1);
        let start_y = ((72 - actual_h as i32) / 2).max(1);

        let mut cur_x = start_x;
        for sg in scaled_glyphs {
            for y in 0..sg.height() {
                for x in 0..sg.width() {
                    let p = sg.get_pixel(x, y);
                    let px = cur_x + x as i32;
                    let py = start_y + y as i32;
                    if px >= 0 && px < 72 && py >= 0 && py < 72 {
                        let alpha = p[3] as f32 / 255.0;
                        let r = (p[0] as f32 * alpha) as u8;
                        let g = (p[1] as f32 * alpha) as u8;
                        let b = (p[2] as f32 * alpha) as u8;
                        if r > 0 || g > 0 || b > 0 {
                            base.put_pixel(px as u32, py as u32, Rgb([r, g, b]));
                        }
                    }
                }
            }
            cur_x += sg.width() as i32 + spacing;
        }

        Some(DynamicImage::ImageRgb8(base))
    }

    fn render_label_button(label: &str, status: &str) -> DynamicImage {
        let mut base = RgbImage::from_pixel(72, 72, Rgb([0, 0, 0]));

        if label.contains('/') {
            // Wskaźnik numeru zadania np. "1/1", "2/4" czy "10/12":
            // Renderuj dynamicznie z wygładzonych wektorowych cyfr oraz ukośnika (100% antyaliasing)
            if let Some(composed) = Self::render_composed_indicator(label) {
                return composed;
            }

            let char_count = label.chars().count() as i32;
            let scale = if char_count <= 4 { 2 } else { 1 };
            let char_w = 5 * scale + scale;
            let total_w = char_count * char_w - scale;
            let start_x = ((72 - total_w) / 2).max(2);
            let start_y = (72 - 7 * scale) / 2;
            Self::draw_scaled_text(&mut base, label, start_x, start_y, scale, Rgb([248, 250, 252]));
            return DynamicImage::ImageRgb8(base);
        }

        let (border_col, text_col) = match status {
            "OK" => (Rgb([56, 189, 248]), Rgb([255, 255, 255])),
            "WA" | "DANGER" => (Rgb([239, 68, 68]), Rgb([255, 255, 255])),
            _ => (Rgb([6, 182, 212]), Rgb([255, 255, 255])),
        };

        // Zaokrąglony kafel zadania na czystym czarnym tle OLED (#000000)
        for x in 8..64 {
            base.put_pixel(x, 10, border_col);
            base.put_pixel(x, 61, border_col);
        }
        for y in 14..58 {
            base.put_pixel(6, y, border_col);
            base.put_pixel(65, y, border_col);
        }

        let len = label.chars().count() as i32;
        let scale = if len <= 4 { 2 } else { 1 };
        let char_w = 5 * scale + scale;
        let total_w = len * char_w - scale;
        let start_x = ((72 - total_w) / 2).max(8);
        let start_y = (72 - 7 * scale) / 2;

        Self::draw_scaled_text(&mut base, label, start_x, start_y, scale, text_col);
        DynamicImage::ImageRgb8(base)
    }

    fn draw_scaled_text(img: &mut RgbImage, text: &str, start_x: i32, start_y: i32, scale: i32, color: Rgb<u8>) {
        let mut cur_x = start_x;
        for c in text.chars() {
            let glyph = Self::get_char_glyph(c);
            for (row_idx, row_bits) in glyph.iter().enumerate() {
                for col_idx in 0..5 {
                    if (row_bits & (1 << (4 - col_idx))) != 0 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = cur_x + col_idx as i32 * scale + sx;
                                let py = start_y + row_idx as i32 * scale + sy;
                                if px >= 0 && px < 72 && py >= 0 && py < 72 {
                                    img.put_pixel(px as u32, py as u32, color);
                                }
                            }
                        }
                    }
                }
            }
            cur_x += 5 * scale + scale;
        }
    }

    #[allow(dead_code)]
    fn draw_mini_text(img: &mut RgbImage, text: &str, start_x: i32, start_y: i32, color: Rgb<u8>) {
        let mut cur_x = start_x;
        for c in text.chars() {
            let glyph = Self::get_char_glyph(c);
            for (row_idx, row_bits) in glyph.iter().enumerate() {
                for col_idx in 0..5 {
                    if (row_bits & (1 << (4 - col_idx))) != 0 {
                        let px = cur_x + col_idx as i32;
                        let py = start_y + row_idx as i32;
                        if px >= 0 && px < 72 && py >= 0 && py < 72 {
                            img.put_pixel(px as u32, py as u32, color);
                        }
                    }
                }
            }
            cur_x += 6;
        }
    }

    fn get_char_glyph(c: char) -> [u8; 7] {
        match c.to_ascii_uppercase() {
            'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
            'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
            'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
            'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
            'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
            'G' => [0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
            'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
            'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
            'J' => [0b00001, 0b00001, 0b00001, 0b00001, 0b10001, 0b10001, 0b01110],
            'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
            'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
            'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
            'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
            'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
            'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
            'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
            'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
            'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
            'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
            'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
            'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
            'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
            'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
            'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
            '0' => [0b01110, 0b10011, 0b10101, 0b10101, 0b11001, 0b10001, 0b01110],
            '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
            '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
            '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
            '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
            '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
            '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
            '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
            '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
            '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
            '/' => [0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b00000],
            _   => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        }
    }

    pub fn start_hardware_thread(
        self: Arc<Self>,
        action_callback: Arc<dyn Fn(String, usize) + Send + Sync>,
    ) {
        std::thread::spawn(move || {
            let elgato_vid = 0x0fd9;
            println!("🎮 Inicjalizacja sterownika sprzętowego Stream Deck (100% Rust USB HID)...");
            kill_streamdeck_conflicts();

            while self.running.load(Ordering::Relaxed) {
                let probes = match StreamDeck::probe() {
                    Ok(p) => p,
                    Err(_) => {
                        std::thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                };

                let mut connected_device: Option<StreamDeck> = None;
                for res in probes {
                    if let Ok((_kind, pid)) = res {
                        if let Ok(deck) = StreamDeck::connect(elgato_vid, pid, None) {
                            connected_device = Some(deck);
                            break;
                        }
                    }
                }

                if let Some(mut deck) = connected_device {
                    println!("✅ Połączono z fizycznym kontrolerem Elgato Stream Deck przez USB!");
                    {
                        let mut state = self.state.lock().unwrap();
                        state.is_hardware_connected = true;
                    }

                    let mut current_brightness = self.brightness.load(Ordering::Relaxed);
                    let _ = deck.reset();
                    let _ = deck.set_brightness(current_brightness);

                    // Pełne wyrenderowanie 15 klawiszy na starcie
                    {
                        let state = self.state.lock().unwrap();
                        for key in &state.keys {
                            let dyn_img = self.render_key_image(key);
                            let _ = deck.set_button_image(key.id as u8, dyn_img);
                        }
                    }
                    self.dirty.store(false, Ordering::SeqCst);

                    let mut last_buttons = vec![0u8; 15];

                    while self.running.load(Ordering::Relaxed) {
                        let new_br = self.brightness.load(Ordering::Relaxed);
                        if new_br != current_brightness {
                            let _ = deck.set_brightness(new_br);
                            current_brightness = new_br;
                        }

                        if self.dirty.swap(false, Ordering::SeqCst) {
                            let state = self.state.lock().unwrap();
                            for key in &state.keys {
                                let dyn_img = self.render_key_image(key);
                                let _ = deck.set_button_image(key.id as u8, dyn_img);
                            }
                        }

                        match deck.read_buttons(Some(Duration::from_millis(40))) {
                            Ok(current_buttons) => {
                                for i in 0..current_buttons.len().min(15) {
                                    if current_buttons[i] == 1 && last_buttons.get(i).copied().unwrap_or(0) == 0 {
                                        let action = {
                                            let state = self.state.lock().unwrap();
                                            state.keys[i].action.clone()
                                        };
                                        if !action.is_empty() {
                                            let cb = action_callback.clone();
                                            std::thread::spawn(move || {
                                                cb(action, i);
                                            });
                                        }
                                    }
                                }
                                last_buttons = current_buttons;
                            }
                            Err(streamdeck::Error::NoData) => {}
                            Err(e) => {
                                eprintln!("⚠️ Utracono połączenie ze Stream Deckiem: {:?}", e);
                                break;
                            }
                        }
                    }

                    // Wygaszenie i zresetowanie wszystkich 15 klawiszy do czerni przed odłączeniem
                    let black_img = DynamicImage::ImageRgb8(RgbImage::new(72, 72));
                    for i in 0..15 {
                        let _ = deck.set_button_image(i as u8, black_img.clone());
                    }
                    let _ = deck.reset();

                    {
                        let mut state = self.state.lock().unwrap();
                        state.is_hardware_connected = false;
                    }
                } else {
                    std::thread::sleep(Duration::from_secs(2));
                }
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Bezpośrednie połączenie z fizycznym kontrolerem USB HID i wyczyszczenie całego ekranu do czerni
    pub fn blank_device_if_connected() {
        let elgato_vid = 0x0fd9;
        if let Ok(probes) = StreamDeck::probe() {
            for res in probes {
                if let Ok((_kind, pid)) = res {
                    if let Ok(mut deck) = StreamDeck::connect(elgato_vid, pid, None) {
                        let black_img = DynamicImage::ImageRgb8(RgbImage::new(72, 72));
                        for i in 0..15 {
                            let _ = deck.set_button_image(i as u8, black_img.clone());
                        }
                        let _ = deck.reset();
                        break;
                    }
                }
            }
        }
    }
}

pub fn kill_streamdeck_conflicts() {
    let mut killed = false;

    #[cfg(target_os = "windows")]
    {
        let conflict_binaries = [
            "StreamDeck.exe",
            "StreamController.exe",
            "deckmaster.exe",
            "boatswain.exe",
            "streamdeck-ui.exe",
            "opendeck.exe",
        ];
        for bin in &conflict_binaries {
            let status = std::process::Command::new("taskkill")
                .args(["/F", "/IM", bin, "/T"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            if let Ok(s) = status {
                if s.success() {
                    eprintln!("\x1b[1;33m⚡ [AlgoDeck] Wymuszono zamknięcie kolidującego programu: {}\x1b[0m", bin);
                    killed = true;
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Najpierw zatrzymaj systemd user services, aby systemd ich nie zrestartował
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "stop", "streamcontroller.service"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "stop", "com.core447.StreamController.service"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        // Zatrzymaj wersję Flatpak jeśli istnieje
        let _ = std::process::Command::new("flatpak")
            .args(["kill", "com.core447.StreamController"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        let my_pid = std::process::id();
        let conflict_patterns = [
            "streamcontroller",
            "StreamController",
            "boatswain",
            "deckmaster",
            "streamdeck-ui",
            "opendeck",
        ];

        for pattern in &conflict_patterns {
            let pgrep = std::process::Command::new("pgrep")
                .args(["-f", pattern])
                .output();
            if let Ok(out) = pgrep {
                let pids: Vec<u32> = String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .filter_map(|l| l.trim().parse::<u32>().ok())
                    .filter(|&pid| pid != my_pid)
                    .collect();

                if !pids.is_empty() {
                    for pid in &pids {
                        let _ = std::process::Command::new("kill")
                            .args(["-9", &pid.to_string()])
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .status();
                    }
                    eprintln!("\x1b[1;33m⚡ [AlgoDeck] Wymuszono zamknięcie kolidującego programu: {} (PID: {:?})\x1b[0m", pattern, pids);
                    killed = true;
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let my_pid = std::process::id();
        let conflict_patterns = [
            "Stream Deck",
            "StreamDeck",
            "streamcontroller",
            "StreamController",
            "deckmaster",
            "boatswain",
            "streamdeck-ui",
        ];

        for pattern in &conflict_patterns {
            let pgrep = std::process::Command::new("pgrep")
                .args(["-f", pattern])
                .output();
            if let Ok(out) = pgrep {
                let pids: Vec<u32> = String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .filter_map(|l| l.trim().parse::<u32>().ok())
                    .filter(|&pid| pid != my_pid)
                    .collect();

                if !pids.is_empty() {
                    for pid in &pids {
                        let _ = std::process::Command::new("kill")
                            .args(["-9", &pid.to_string()])
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .status();
                    }
                    eprintln!("\x1b[1;33m⚡ [AlgoDeck] Wymuszono zamknięcie kolidującego programu: {} (PID: {:?})\x1b[0m", pattern, pids);
                    killed = true;
                }
            }
        }
    }

    if killed {
        // Krótka pauza, aby deskryptory USB HID zostały w pełni zwolnione w systemie operacyjnym
        std::thread::sleep(std::time::Duration::from_millis(300));
        eprintln!("\x1b[1;32m✓ [AlgoDeck] Zwolniono urządzenie Stream Deck dla AlgoDeck.\x1b[0m\n");
    }
}
