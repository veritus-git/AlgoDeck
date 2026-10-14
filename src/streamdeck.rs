use image::{DynamicImage, Rgb, RgbImage};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use streamdeck::StreamDeck;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamDeckKey {
    pub id: usize,
    pub label: String,
    pub icon: String,
    pub subtext: String,
    pub status: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDeckState {
    pub keys: Vec<StreamDeckKey>,
    pub active_problem_id: Option<String>,
    pub is_hardware_connected: bool,
}

pub struct StreamDeckEngine {
    state: Arc<Mutex<StreamDeckState>>,
    running: Arc<AtomicBool>,
    dirty: Arc<AtomicBool>,
}

impl StreamDeckEngine {
    pub fn new() -> Self {
        let mut keys = Vec::with_capacity(15);
        for i in 0..15 {
            keys.push(StreamDeckKey {
                id: i,
                label: String::new(),
                icon: String::new(),
                subtext: String::new(),
                status: "IDLE".to_string(),
                action: String::new(),
            });
        }

        let initial_state = StreamDeckState {
            keys,
            active_problem_id: None,
            is_hardware_connected: false,
        };

        let engine = Self {
            state: Arc::new(Mutex::new(initial_state)),
            running: Arc::new(AtomicBool::new(true)),
            dirty: Arc::new(AtomicBool::new(true)),
        };

        engine.set_idle_screen();
        engine
    }

    pub fn get_state(&self) -> StreamDeckState {
        self.state.lock().unwrap().clone()
    }

    pub fn set_idle_screen(&self) {
        let mut state = self.state.lock().unwrap();
        state.active_problem_id = None;
        for i in 0..15 {
            state.keys[i] = StreamDeckKey {
                id: i,
                label: String::new(),
                icon: String::new(),
                subtext: String::new(),
                status: "IDLE".to_string(),
                action: String::new(),
            };
        }

        // Welcome screen layout
        state.keys[2] = StreamDeckKey {
            id: 2,
            label: "MAP".to_string(),
            icon: "●".to_string(),
            subtext: "OI".to_string(),
            status: "OK".to_string(),
            action: String::new(),
        };

        state.keys[7] = StreamDeckKey {
            id: 7,
            label: "ALGODECK".to_string(),
            icon: "⚡".to_string(),
            subtext: "START".to_string(),
            status: "ACTION".to_string(),
            action: "open_menu".to_string(),
        };

        self.dirty.store(true, Ordering::SeqCst);
    }

    pub fn set_active_problem(&self, problem_id: &str) {
        let pid = problem_id.to_lowercase();
        let mut state = self.state.lock().unwrap();
        state.active_problem_id = Some(pid.clone());

        for i in 0..15 {
            state.keys[i] = StreamDeckKey {
                id: i,
                label: String::new(),
                icon: String::new(),
                subtext: String::new(),
                status: "IDLE".to_string(),
                action: String::new(),
            };
        }

        // Key 2: Problem Badge
        state.keys[2] = StreamDeckKey {
            id: 2,
            label: pid.to_uppercase(),
            icon: "●".to_string(),
            subtext: "OI".to_string(),
            status: "OK".to_string(),
            action: "notify_task".to_string(),
        };

        // Middle Row (5..9): Action Keys
        state.keys[5] = StreamDeckKey {
            id: 5,
            label: "CODE".to_string(),
            icon: "</>".to_string(),
            subtext: "VS".to_string(),
            status: "ACTION".to_string(),
            action: "open_vscode".to_string(),
        };
        state.keys[6] = StreamDeckKey {
            id: 6,
            label: "TEST".to_string(),
            icon: "⚗".to_string(),
            subtext: "ALL".to_string(),
            status: "OK".to_string(),
            action: "run_all".to_string(),
        };
        state.keys[7] = StreamDeckKey {
            id: 7,
            label: "RUN".to_string(),
            icon: "▶".to_string(),
            subtext: "CLI".to_string(),
            status: "DEBUG".to_string(),
            action: "run_interactive".to_string(),
        };
        state.keys[8] = StreamDeckKey {
            id: 8,
            label: "KILL".to_string(),
            icon: "⏹".to_string(),
            subtext: "STOP".to_string(),
            status: "DANGER".to_string(),
            action: "kill_process".to_string(),
        };
        state.keys[9] = StreamDeckKey {
            id: 9,
            label: "MENU".to_string(),
            icon: "⊞".to_string(),
            subtext: "GUI".to_string(),
            status: "IDLE".to_string(),
            action: "open_menu".to_string(),
        };

        // Bottom Row: Navigation
        state.keys[10] = StreamDeckKey {
            id: 10,
            label: "PREV".to_string(),
            icon: "‹".to_string(),
            subtext: "".to_string(),
            status: "IDLE".to_string(),
            action: "prev_task".to_string(),
        };
        state.keys[14] = StreamDeckKey {
            id: 14,
            label: "NEXT".to_string(),
            icon: "›".to_string(),
            subtext: "".to_string(),
            status: "IDLE".to_string(),
            action: "next_task".to_string(),
        };

        self.dirty.store(true, Ordering::SeqCst);
    }

    pub fn update_key_status(&self, key_id: usize, status: &str, subtext: Option<&str>) {
        let mut state = self.state.lock().unwrap();
        if key_id < 15 {
            state.keys[key_id].status = status.to_string();
            if let Some(sub) = subtext {
                state.keys[key_id].subtext = sub.to_string();
            }
            self.dirty.store(true, Ordering::SeqCst);
        }
    }

    // --- 5x7 BITMAP FONT RENDERER FOR CRISP 72x72 LCD BUTTONS ---
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
            '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
            '>' => [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
            '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
            '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
            '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
            ':' => [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000],
            '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100],
            _   => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        }
    }

    fn draw_text(img: &mut RgbImage, text: &str, start_x: i32, start_y: i32, scale: u32, color: Rgb<u8>) {
        let mut cur_x = start_x;
        for c in text.chars() {
            let glyph = Self::get_char_glyph(c);
            for (row_idx, row_bits) in glyph.iter().enumerate() {
                for col_idx in 0..5 {
                    if (row_bits & (1 << (4 - col_idx))) != 0 {
                        for sx in 0..scale {
                            for sy in 0..scale {
                                let px = cur_x + (col_idx as i32 * scale as i32) + sx as i32;
                                let py = start_y + (row_idx as i32 * scale as i32) + sy as i32;
                                if px >= 0 && px < 72 && py >= 0 && py < 72 {
                                    img.put_pixel(px as u32, py as u32, color);
                                }
                            }
                        }
                    }
                }
            }
            cur_x += (5 * scale + scale) as i32;
        }
    }

    fn get_text_width(text: &str, scale: u32) -> i32 {
        if text.is_empty() {
            return 0;
        }
        let len = text.chars().count() as i32;
        len * (5 * scale as i32) + (len - 1) * (scale as i32)
    }

    pub fn render_key_image(key: &StreamDeckKey) -> DynamicImage {
        let size = 72u32;
        let mut img = RgbImage::new(size, size);

        // Paleta kolorów
        let (bg_color, border_color) = match key.status.as_str() {
            "ACTION" => (Rgb([10, 28, 50]), Rgb([2, 132, 199])),
            "OK" => (Rgb([6, 78, 59]), Rgb([16, 215, 125])),
            "WA" => (Rgb([127, 29, 29]), Rgb([239, 68, 68])),
            "CE" | "DANGER" => (Rgb([153, 27, 27]), Rgb([239, 68, 68])),
            "RUNNING" => (Rgb([113, 63, 18]), Rgb([234, 179, 8])),
            "DEBUG" => (Rgb([23, 37, 84]), Rgb([56, 189, 248])),
            _ => (Rgb([17, 24, 39]), Rgb([55, 65, 81])),
        };

        // 1. Wypełnienie tła
        for x in 0..size {
            for y in 0..size {
                img.put_pixel(x, y, bg_color);
            }
        }

        // 2. Elegancka zaokrąglona ramka (2px)
        for x in 2..(size - 2) {
            img.put_pixel(x, 1, border_color);
            img.put_pixel(x, size - 2, border_color);
        }
        for y in 2..(size - 2) {
            img.put_pixel(1, y, border_color);
            img.put_pixel(size - 2, y, border_color);
        }

        let white = Rgb([248, 250, 252]);
        let accent = border_color;
        let text_muted = Rgb([203, 213, 225]);

        let cx = size / 2;
        let cy = size / 2;

        // 3. Rysowanie ikony w centrum
        match key.icon.as_str() {
            "▶" => {
                for dx in -7i32..=7 {
                    let height = 7 - dx.abs();
                    for dy in -height..=height {
                        let px = (cx as i32 + dx + 2) as u32;
                        let py = (cy as i32 + dy) as u32;
                        if px < size && py < size {
                            img.put_pixel(px, py, white);
                        }
                    }
                }
            }
            "⏹" => {
                for dx in -7i32..=7 {
                    for dy in -7i32..=7 {
                        let px = (cx as i32 + dx) as u32;
                        let py = (cy as i32 + dy) as u32;
                        if px < size && py < size {
                            img.put_pixel(px, py, white);
                        }
                    }
                }
            }
            "●" => {
                for dx in -8i32..=8 {
                    for dy in -8i32..=8 {
                        if dx * dx + dy * dy <= 64 {
                            let px = (cx as i32 + dx) as u32;
                            let py = (cy as i32 + dy) as u32;
                            if px < size && py < size {
                                img.put_pixel(px, py, accent);
                            }
                        }
                    }
                }
            }
            "⊞" => {
                for dx in -8i32..=8 {
                    for dy in -8i32..=8 {
                        if dx != 0 && dy != 0 && (dx.abs() <= 7 && dy.abs() <= 7) {
                            let px = (cx as i32 + dx) as u32;
                            let py = (cy as i32 + dy) as u32;
                            if px < size && py < size {
                                img.put_pixel(px, py, white);
                            }
                        }
                    }
                }
            }
            "‹" => {
                for d in -7i32..=7 {
                    let px = (cx as i32 - 4 + d.abs()) as u32;
                    let py = (cy as i32 + d) as u32;
                    if px < size && py < size {
                        img.put_pixel(px, py, white);
                        img.put_pixel(px + 1, py, white);
                    }
                }
            }
            "›" => {
                for d in -7i32..=7 {
                    let px = (cx as i32 + 4 - d.abs()) as u32;
                    let py = (cy as i32 + d) as u32;
                    if px < size && py < size {
                        img.put_pixel(px, py, white);
                        img.put_pixel(px - 1, py, white);
                    }
                }
            }
            "⚡" => {
                for y in 0..16 {
                    let x = if y < 8 { cx - 4 + y / 2 } else { cx + 4 - (y - 8) / 2 };
                    let py = cy - 8 + y;
                    for w in 0..4 {
                        if x + w < size && py < size {
                            img.put_pixel(x + w, py, Rgb([250, 204, 21]));
                        }
                    }
                }
            }
            "</>" => {
                Self::draw_text(&mut img, "</>", 24, 32, 1, accent);
            }
            "⚗" => {
                // Rysunek kolby laboratoryjnej
                for dy in -7i32..=7 {
                    let w = if dy < -2 { 2 } else { 3 + (dy + 2) };
                    for dx in -w..=w {
                        let px = (cx as i32 + dx) as u32;
                        let py = (cy as i32 + dy) as u32;
                        if px < size && py < size {
                            img.put_pixel(px, py, accent);
                        }
                    }
                }
            }
            _ => {}
        }

        // 4. Etykieta na górze (Label)
        if !key.label.is_empty() {
            let width = Self::get_text_width(&key.label, 1);
            let start_x = (72 - width) / 2;
            Self::draw_text(&mut img, &key.label, start_x.max(4), 6, 1, white);
        }

        // 5. Podpis na dole (Subtext)
        if !key.subtext.is_empty() {
            let width = Self::get_text_width(&key.subtext, 1);
            let start_x = (72 - width) / 2;
            Self::draw_text(&mut img, &key.subtext, start_x.max(4), 58, 1, text_muted);
        }

        DynamicImage::ImageRgb8(img)
    }

    pub fn start_hardware_thread(
        self: Arc<Self>,
        action_callback: Arc<dyn Fn(String, usize) + Send + Sync>,
    ) {
        std::thread::spawn(move || {
            let elgato_vid = 0x0fd9;
            println!("🎮 Inicjalizacja sterownika sprzętowego Stream Deck (100% Rust USB HID)...");

            while self.running.load(Ordering::Relaxed) {
                // Wykrywanie urządzenia
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

                    // Pełne wyrenderowanie wszystkich 15 klawiszy na starcie połączenia
                    {
                        let state = self.state.lock().unwrap();
                        for key in &state.keys {
                            let dyn_img = Self::render_key_image(key);
                            let _ = deck.set_button_image(key.id as u8, dyn_img);
                        }
                    }
                    self.dirty.store(false, Ordering::SeqCst);

                    let mut last_buttons = vec![0u8; 15];

                    while self.running.load(Ordering::Relaxed) {
                        // Aktualizuj klawisze tylko gdy stan uległ zmianie (dirty tracking)
                        if self.dirty.swap(false, Ordering::SeqCst) {
                            let state = self.state.lock().unwrap();
                            for key in &state.keys {
                                let dyn_img = Self::render_key_image(key);
                                let _ = deck.set_button_image(key.id as u8, dyn_img);
                            }
                        }

                        // Odczyt wciśnięć przycisków z timeoutem 40ms
                        match deck.read_buttons(Some(Duration::from_millis(40))) {
                            Ok(current_buttons) => {
                                for i in 0..current_buttons.len().min(15) {
                                    // Zbocze narastające przycisku (wciśnięcie)
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
                            Err(streamdeck::Error::NoData) => {
                                // Timeout, stan normalny
                            }
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
                    // Brak fizycznego kontrolera - spróbuj ponownie za 2 sekundy
                    std::thread::sleep(Duration::from_secs(2));
                }
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
