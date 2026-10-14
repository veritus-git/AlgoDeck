use image::{imageops, DynamicImage, Rgb, RgbImage};
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
            icon_cache: Arc::new(Mutex::new(HashMap::new())),
        };

        engine.set_idle_screen();
        engine
    }

    pub fn get_state(&self) -> StreamDeckState {
        self.state.lock().unwrap().clone()
    }

    fn find_icon_file(name: &str) -> Option<PathBuf> {
        let home_icons = dirs::home_dir().map(|h| h.join(".local/share/algodeck/icons").join(name));
        let candidates = [
            home_icons,
            Some(PathBuf::from("/home/linux/.local/share/algodeck/icons").join(name)),
            Some(PathBuf::from("/home/linux/.gemini/antigravity-ide/scratch/algodeck/frontend/icons").join(name)),
            dirs::data_dir().map(|d| d.join("algodeck/icons").join(name)),
            Some(PathBuf::from("frontend/icons").join(name)),
            Some(PathBuf::from("/usr/local/share/algodeck/icons").join(name)),
            Some(PathBuf::from("/usr/share/algodeck/icons").join(name)),
        ];

        for c in candidates.into_iter().flatten() {
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
        state.keys[11] = StreamDeckKey {
            id: 11,
            icon_name: "icon_exit.png".to_string(),
            action: "exit_to_idle".to_string(),
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

    pub fn set_active_problem(&self, problem_id: &str) {
        self.set_active_problem_with_meta(problem_id, 1, 1);
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
        // Jeśli mamy bezpośredni plik ikony, wczytaj go
        if !key.icon_name.is_empty() {
            let img = self.load_icon(&key.icon_name);
            // Jeśli ikona istnieje i nie jest pustym czarnym kwadratem, zwróć ją
            if Self::is_non_black(&img) {
                return img;
            }
        }

        // Jeśli klawisz ma custom_label (np. w menu zadań lub numerze 1/4)
        if let Some(ref label) = key.custom_label {
            return Self::render_label_button(label, &key.status);
        }

        // Domyślny całkowicie czarny ekran OLED (#000000)
        DynamicImage::ImageRgb8(RgbImage::from_pixel(72, 72, Rgb([0, 0, 0])))
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

    fn render_label_button(label: &str, status: &str) -> DynamicImage {
        let mut base = RgbImage::from_pixel(72, 72, Rgb([0, 0, 0]));

        if label.contains('/') {
            // Wskaźnik numeru zadania np. "1/5": czysty tekst wycentrowany na PURE BLACK #000000 bez ramki
            let len = label.chars().count() as i32;
            let char_w = 6;
            let start_x = ((72 - (len * char_w)) / 2).max(2);
            let start_y = 32;
            Self::draw_mini_text(&mut base, label, start_x, start_y, Rgb([226, 232, 240]));
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
        let char_w = 6;
        let start_x = ((72 - (len * char_w)) / 2).max(6);
        let start_y = 33;

        Self::draw_mini_text(&mut base, label, start_x, start_y, text_col);
        DynamicImage::ImageRgb8(base)
    }

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

                    let _ = deck.reset();
                    let _ = deck.set_brightness(85);

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
                                            action_callback(action, i);
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
}
