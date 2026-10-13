use image::{DynamicImage, Rgb, RgbImage};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use streamdeck::StreamDeck;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

        Self {
            state: Arc::new(Mutex::new(initial_state)),
            running: Arc::new(AtomicBool::new(true)),
        }
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
        state.keys[7] = StreamDeckKey {
            id: 7,
            label: "ALGODECK".to_string(),
            icon: "⚡".to_string(),
            subtext: "READY".to_string(),
            status: "ACTION".to_string(),
            action: "open_menu".to_string(),
        };
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
            subtext: String::new(),
            status: "OK".to_string(),
            action: "notify_task".to_string(),
        };

        // Middle Row (5..9): Action Keys
        state.keys[5] = StreamDeckKey {
            id: 5,
            label: "CODE".to_string(),
            icon: "</>".to_string(),
            subtext: String::new(),
            status: "ACTION".to_string(),
            action: "open_vscode".to_string(),
        };
        state.keys[6] = StreamDeckKey {
            id: 6,
            label: "TEST".to_string(),
            icon: "⚗".to_string(),
            subtext: String::new(),
            status: "OK".to_string(),
            action: "run_all".to_string(),
        };
        state.keys[7] = StreamDeckKey {
            id: 7,
            label: "RUN".to_string(),
            icon: "▶".to_string(),
            subtext: String::new(),
            status: "DEBUG".to_string(),
            action: "run_interactive".to_string(),
        };
        state.keys[8] = StreamDeckKey {
            id: 8,
            label: "KILL".to_string(),
            icon: "⏹".to_string(),
            subtext: String::new(),
            status: "DANGER".to_string(),
            action: "kill_process".to_string(),
        };
        state.keys[9] = StreamDeckKey {
            id: 9,
            label: "MENU".to_string(),
            icon: "⊞".to_string(),
            subtext: String::new(),
            status: "IDLE".to_string(),
            action: "open_menu".to_string(),
        };

        // Bottom Row: Navigation
        state.keys[10] = StreamDeckKey {
            id: 10,
            label: String::new(),
            icon: "‹".to_string(),
            subtext: String::new(),
            status: "IDLE".to_string(),
            action: "prev_task".to_string(),
        };
        state.keys[14] = StreamDeckKey {
            id: 14,
            label: String::new(),
            icon: "›".to_string(),
            subtext: String::new(),
            status: "IDLE".to_string(),
            action: "next_task".to_string(),
        };
    }

    pub fn update_key_status(&self, key_id: usize, status: &str, subtext: Option<&str>) {
        let mut state = self.state.lock().unwrap();
        if key_id < 15 {
            state.keys[key_id].status = status.to_string();
            if let Some(sub) = subtext {
                state.keys[key_id].subtext = sub.to_string();
            }
        }
    }

    pub fn render_key_image(key: &StreamDeckKey) -> DynamicImage {
        let size = 72u32;
        let mut img = RgbImage::new(size, size);

        let bg_color = match key.status.as_str() {
            "ACTION" => Rgb([14, 30, 52]),
            "OK" => Rgb([6, 78, 59]),
            "WA" => Rgb([127, 29, 29]),
            "CE" | "DANGER" => Rgb([153, 27, 27]),
            "RUNNING" => Rgb([113, 63, 18]),
            "DEBUG" => Rgb([30, 58, 138]),
            _ => Rgb([17, 24, 39]),
        };

        let border_color = match key.status.as_str() {
            "ACTION" => Rgb([2, 132, 199]),
            "OK" => Rgb([16, 185, 129]),
            "WA" | "CE" | "DANGER" => Rgb([239, 68, 68]),
            "RUNNING" => Rgb([234, 179, 8]),
            "DEBUG" => Rgb([56, 189, 248]),
            _ => Rgb([55, 65, 81]),
        };

        // Fill background
        for x in 0..size {
            for y in 0..size {
                img.put_pixel(x, y, bg_color);
            }
        }

        // Draw border
        for x in 0..size {
            img.put_pixel(x, 0, border_color);
            img.put_pixel(x, 1, border_color);
            img.put_pixel(x, size - 1, border_color);
            img.put_pixel(x, size - 2, border_color);
        }
        for y in 0..size {
            img.put_pixel(0, y, border_color);
            img.put_pixel(1, y, border_color);
            img.put_pixel(size - 1, y, border_color);
            img.put_pixel(size - 2, y, border_color);
        }

        // Simple pixel glyphs for icons in the center
        let cx = size / 2;
        let cy = size / 2;
        let white = Rgb([248, 250, 252]);
        let accent = border_color;

        match key.icon.as_str() {
            "▶" => {
                // Play triangle
                for dx in -6i32..=6 {
                    let height = 6 - dx.abs();
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
                // Stop square
                for dx in -6i32..=6 {
                    for dy in -6i32..=6 {
                        let px = (cx as i32 + dx) as u32;
                        let py = (cy as i32 + dy) as u32;
                        if px < size && py < size {
                            img.put_pixel(px, py, white);
                        }
                    }
                }
            }
            "●" => {
                // Circle dot
                for dx in -7i32..=7 {
                    for dy in -7i32..=7 {
                        if dx * dx + dy * dy <= 49 {
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
                // Menu grid 2x2
                for dx in -7i32..=7 {
                    for dy in -7i32..=7 {
                        if dx != 0 && dy != 0 {
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
                // Left chevron
                for d in -6i32..=6 {
                    let px = (cx as i32 - 4 + d.abs()) as u32;
                    let py = (cy as i32 + d) as u32;
                    if px < size && py < size {
                        img.put_pixel(px, py, white);
                        img.put_pixel(px + 1, py, white);
                    }
                }
            }
            "›" => {
                // Right chevron
                for d in -6i32..=6 {
                    let px = (cx as i32 + 4 - d.abs()) as u32;
                    let py = (cy as i32 + d) as u32;
                    if px < size && py < size {
                        img.put_pixel(px, py, white);
                        img.put_pixel(px - 1, py, white);
                    }
                }
            }
            "⚡" => {
                // Lightning bolt
                for y in 0..14 {
                    let x = if y < 7 { cx - 3 + y / 2 } else { cx + 3 - (y - 7) / 2 };
                    let py = cy - 7 + y;
                    for w in 0..3 {
                        if x + w < size && py < size {
                            img.put_pixel(x + w, py, Rgb([250, 204, 21]));
                        }
                    }
                }
            }
            _ => {
                // Draw a sleek indicator dot or square
                if !key.label.is_empty() {
                    for dx in -4i32..=4 {
                        for dy in -4i32..=4 {
                            let px = (cx as i32 + dx) as u32;
                            let py = (cy as i32 + dy) as u32;
                            if px < size && py < size {
                                img.put_pixel(px, py, accent);
                            }
                        }
                    }
                }
            }
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
                // Probe for devices
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
                    let _ = deck.set_brightness(80);

                    // Render current keys
                    {
                        let state = self.state.lock().unwrap();
                        for key in &state.keys {
                            let dyn_img = Self::render_key_image(key);
                            let _ = deck.set_button_image(key.id as u8, dyn_img);
                        }
                    }

                    let mut last_buttons = vec![0u8; 15];

                    while self.running.load(Ordering::Relaxed) {
                        // Check if key states need re-rendering
                        {
                            let state = self.state.lock().unwrap();
                            for key in &state.keys {
                                let dyn_img = Self::render_key_image(key);
                                let _ = deck.set_button_image(key.id as u8, dyn_img);
                            }
                        }

                        // Read button presses with 60ms timeout
                        match deck.read_buttons(Some(Duration::from_millis(60))) {
                            Ok(current_buttons) => {
                                for i in 0..current_buttons.len().min(15) {
                                    // Trigger on button down edge: last was 0, now is 1
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
                                // Timeout, normal
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
                    // No hardware detected right now
                    std::thread::sleep(Duration::from_secs(2));
                }
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
