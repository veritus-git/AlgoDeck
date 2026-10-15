use algodeck::config::AppSettings;
use algodeck::runner::TestExecutor;
use algodeck::server::{create_router, AppState};
use algodeck::streamdeck::StreamDeckEngine;
use image::{GenericImage, Rgb, RgbImage};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::broadcast;

fn count_non_black_pixels(img: &image::DynamicImage) -> usize {
    let rgb = img.to_rgb8();
    rgb.pixels().filter(|p| p[0] > 10 || p[1] > 10 || p[2] > 10).count()
}

#[test]
fn test_streamdeck_idle_screen_rendering() {
    let engine = StreamDeckEngine::new();
    engine.set_idle_screen();
    let state = engine.get_state();

    assert_eq!(state.keys.len(), 15);
    // Środkowe kafelki: 6 (ALGO), 7 (BOLT), 8 (DECK)
    assert_eq!(state.keys[6].action, "start_or_resume");
    assert_eq!(state.keys[7].action, "start_or_resume");
    assert_eq!(state.keys[8].action, "start_or_resume");

    let img_algo = engine.render_key_image(&state.keys[6]);
    let img_bolt = engine.render_key_image(&state.keys[7]);
    let img_deck = engine.render_key_image(&state.keys[8]);

    assert!(count_non_black_pixels(&img_algo) > 50, "Kafel ALGO musi zawierać piksele");
    assert!(count_non_black_pixels(&img_bolt) > 50, "Kafel BOLT musi zawierać piksele");
    assert!(count_non_black_pixels(&img_deck) > 50, "Kafel DECK musi zawierać piksele");
}

#[test]
fn test_streamdeck_task_screen_rendering() {
    let engine = StreamDeckEngine::new();
    engine.set_active_problem_with_meta("sil", 1, 3);
    let state = engine.get_state();

    assert_eq!(state.active_problem_id.as_deref(), Some("sil"));
    assert_eq!(state.keys.len(), 15);

    // Klawisze:
    // 0: '='
    // 1: 's'
    // 2: 'i'
    // 3: 'l'
    // 4: '='
    // 5: '+' (new_task)
    // 6: run_all
    // 7: run_interactive
    // 8: kill_process
    // 9: open_menu
    // 10: open_settings
    // 12: prev_task
    // 13: 1/3 (numer)
    // 14: next_task

    let active_keys = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14];
    for &k in &active_keys {
        let img = engine.render_key_image(&state.keys[k]);
        let non_black = count_non_black_pixels(&img);
        assert!(
            non_black > 30,
            "Klawisz {} ({}) nie może być pusty/czarny! Liczba pikseli: {}",
            k,
            state.keys[k].icon_name,
            non_black
        );
    }

    // Klawisz 11 jest zamierzenie pusty (czysty czarny OLED)
    let empty_img = engine.render_key_image(&state.keys[11]);
    assert_eq!(count_non_black_pixels(&empty_img), 0, "Klawisz 11 musi być czysty czarny");
}

#[test]
fn test_procedural_fallback_tiles_render_properly() {
    let tile_plus = StreamDeckEngine::render_fallback_tile("icon_plus.png", "new_task", "IDLE");
    assert!(count_non_black_pixels(&tile_plus) > 50);

    let tile_algo = StreamDeckEngine::render_fallback_tile("idle_logo_algo.png", "start_or_resume", "IDLE");
    assert!(count_non_black_pixels(&tile_algo) > 50);

    let tile_bolt = StreamDeckEngine::render_fallback_tile("idle_logo_bolt.png", "start_or_resume", "IDLE");
    assert!(count_non_black_pixels(&tile_bolt) > 50);

    let tile_letter = StreamDeckEngine::render_fallback_tile("letter_z.png", "open_vscode", "IDLE");
    assert!(count_non_black_pixels(&tile_letter) > 40);

    let tile_test = StreamDeckEngine::render_fallback_tile("icon_test_sec.png", "run_all", "OK");
    assert!(count_non_black_pixels(&tile_test) > 50);

    let tile_settings = StreamDeckEngine::render_fallback_tile("icon_settings.png", "open_settings", "IDLE");
    assert!(count_non_black_pixels(&tile_settings) > 50);
}

#[test]
fn test_generate_and_save_streamdeck_visual_snapshot() {
    let engine = StreamDeckEngine::new();
    let btn_w: u32 = 72;
    let btn_h: u32 = 72;
    let gap: u32 = 8;
    let grid_w = 5 * btn_w + 6 * gap;
    let grid_h = 3 * btn_h + 4 * gap;

    std::fs::create_dir_all("target").unwrap();

    // 1. Snapshot ekranu zadania (ALGO_TASK: sil 2/4)
    engine.set_active_problem_with_meta("sil", 2, 4);
    let mut task_canvas = RgbImage::from_pixel(grid_w, grid_h, Rgb([15, 23, 42]));
    let state_task = engine.get_state();
    for row in 0..3u32 {
        for col in 0..5u32 {
            let key_idx = (row * 5 + col) as usize;
            let dyn_img = engine.render_key_image(&state_task.keys[key_idx]);
            let x = gap + col * (btn_w + gap);
            let y = gap + row * (btn_h + gap);
            task_canvas.copy_from(&dyn_img.to_rgb8(), x, y).unwrap();
        }
    }
    task_canvas.save("target/streamdeck_task_grid_snapshot.png").unwrap();

    // 2. Snapshot ekranu powitalnego (ALGO_IDLE)
    engine.set_idle_screen();
    let mut idle_canvas = RgbImage::from_pixel(grid_w, grid_h, Rgb([15, 23, 42]));
    let state_idle = engine.get_state();
    for row in 0..3u32 {
        for col in 0..5u32 {
            let key_idx = (row * 5 + col) as usize;
            let dyn_img = engine.render_key_image(&state_idle.keys[key_idx]);
            let x = gap + col * (btn_w + gap);
            let y = gap + row * (btn_h + gap);
            idle_canvas.copy_from(&dyn_img.to_rgb8(), x, y).unwrap();
        }
    }
    idle_canvas.save("target/streamdeck_idle_grid_snapshot.png").unwrap();

    // 3. Snapshot galerii zadań (ALGO_MENU)
    let pids = vec!["sil".to_string(), "chw".to_string(), "kol".to_string(), "drz".to_string()];
    engine.set_menu_screen(&pids, 0);
    let mut menu_canvas = RgbImage::from_pixel(grid_w, grid_h, Rgb([15, 23, 42]));
    let state_menu = engine.get_state();
    for row in 0..3u32 {
        for col in 0..5u32 {
            let key_idx = (row * 5 + col) as usize;
            let dyn_img = engine.render_key_image(&state_menu.keys[key_idx]);
            let x = gap + col * (btn_w + gap);
            let y = gap + row * (btn_h + gap);
            menu_canvas.copy_from(&dyn_img.to_rgb8(), x, y).unwrap();
        }
    }
    menu_canvas.save("target/streamdeck_menu_grid_snapshot.png").unwrap();

    assert!(std::path::Path::new("target/streamdeck_task_grid_snapshot.png").exists());
    assert!(std::path::Path::new("target/streamdeck_idle_grid_snapshot.png").exists());
    assert!(std::path::Path::new("target/streamdeck_menu_grid_snapshot.png").exists());
}

#[tokio::test]
async fn test_server_status_and_graceful_shutdown() {
    let settings = Arc::new(RwLock::new(AppSettings::default()));
    let runner = Arc::new(TestExecutor::new(settings.clone()));
    let streamdeck = Arc::new(StreamDeckEngine::new());
    let (ws_sender, _) = broadcast::channel(10);
    let shutdown_signal = Arc::new(AtomicBool::new(false));

    let app_state = AppState {
        settings,
        streamdeck,
        runner,
        ws_sender,
        shutdown_signal: shutdown_signal.clone(),
    };

    let router = create_router(app_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("Bind test port");
    let test_addr = listener.local_addr().unwrap();

    let shutdown_rx = shutdown_signal.clone();
    let server_task = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(20));
                loop {
                    interval.tick().await;
                    if shutdown_rx.load(Ordering::Relaxed) {
                        break;
                    }
                }
            })
            .await
            .unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // 1. Sprawdź /api/status
    let status_url = format!("http://{}/api/status", test_addr);
    let resp: serde_json::Value = reqwest_like_get(&status_url).await;
    assert_eq!(resp.get("status").and_then(|v| v.as_str()), Some("ok"));

    // 2. Wyślij shutdown
    let shutdown_url = format!("http://{}/api/shutdown", test_addr);
    let shut_resp: serde_json::Value = reqwest_like_post(&shutdown_url).await;
    assert_eq!(shut_resp.get("success").and_then(|v| v.as_bool()), Some(true));

    // 3. Sprawdź czy shutdown_signal został podniesiony
    assert!(shutdown_signal.load(Ordering::Relaxed), "shutdown_signal musi być true po wywołaniu /api/shutdown");

    // 4. Poczekaj na zakończenie serwera (powinno zakończyć się natychmiast)
    let finished = tokio::time::timeout(Duration::from_millis(500), server_task).await;
    assert!(finished.is_ok(), "Serwer musi się natychmiast wyłączyć po sygnale graceful shutdown!");
}

async fn reqwest_like_get(url: &str) -> serde_json::Value {
    let url_clone = url.to_string();
    tokio::task::spawn_blocking(move || {
        ureq::get(&url_clone).call().unwrap().into_json().unwrap()
    })
    .await
    .unwrap()
}

async fn reqwest_like_post(url: &str) -> serde_json::Value {
    let url_clone = url.to_string();
    tokio::task::spawn_blocking(move || {
        ureq::post(&url_clone).call().unwrap().into_json().unwrap()
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn test_settings_serving_and_brightness_endpoint() {
    let settings = Arc::new(RwLock::new(AppSettings::default()));
    let runner = Arc::new(TestExecutor::new(settings.clone()));
    let streamdeck = Arc::new(StreamDeckEngine::new());
    let (ws_sender, _) = broadcast::channel(10);
    let shutdown_signal = Arc::new(AtomicBool::new(false));

    let app_state = AppState {
        settings,
        runner,
        streamdeck: streamdeck.clone(),
        ws_sender,
        shutdown_signal: shutdown_signal.clone(),
    };

    let router = create_router(app_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("Bind test port");
    let test_addr = listener.local_addr().unwrap();

    let shutdown_rx = shutdown_signal.clone();
    let server_task = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                while !shutdown_rx.load(Ordering::Relaxed) {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // 1. Sprawdź GET /settings
    let settings_url = format!("http://{}/settings", test_addr);
    let html_body = tokio::task::spawn_blocking(move || {
        ureq::get(&settings_url).call().unwrap().into_string().unwrap()
    }).await.unwrap();
    assert!(html_body.contains("AlgoDeck Architecture & Settings"), "Strona settings.html musi zawierać nagłówek AlgoDeck");
    assert!(html_body.contains("id=\"cfg-workspace-dir\""), "Strona musi zawierać pole workspace");
    assert!(html_body.contains("id=\"cfg-sd-brightness\""), "Strona musi zawierać suwak jasności Stream Decka");

    // 2. Sprawdź GET /settings.css
    let css_url = format!("http://{}/settings.css", test_addr);
    let css_resp = tokio::task::spawn_blocking(move || {
        let resp = ureq::get(&css_url).call().unwrap();
        let ctype = resp.header("content-type").unwrap_or("").to_string();
        let body = resp.into_string().unwrap();
        (ctype, body)
    }).await.unwrap();
    assert!(css_resp.0.contains("text/css"), "Content-type dla /settings.css musi być text/css");
    assert!(css_resp.1.contains("--bg-page"), "CSS musi zawierać definicje motywu");

    // 3. Sprawdź GET /settings.js
    let js_url = format!("http://{}/settings.js", test_addr);
    let js_resp = tokio::task::spawn_blocking(move || {
        let resp = ureq::get(&js_url).call().unwrap();
        let ctype = resp.header("content-type").unwrap_or("").to_string();
        let body = resp.into_string().unwrap();
        (ctype, body)
    }).await.unwrap();
    assert!(js_resp.0.contains("javascript"), "Content-type dla /settings.js musi być application/javascript");
    assert!(js_resp.1.contains("API_BASE"), "JS musi zawierać logikę kontrolera");

    // 4. Sprawdź POST /api/streamdeck/brightness
    let br_url = format!("http://{}/api/streamdeck/brightness", test_addr);
    let br_resp = tokio::task::spawn_blocking(move || {
        ureq::post(&br_url)
            .send_json(serde_json::json!({ "brightness": 65 }))
            .unwrap()
            .into_json::<serde_json::Value>()
            .unwrap()
    }).await.unwrap();
    assert_eq!(br_resp.get("success").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(br_resp.get("brightness").and_then(|v| v.as_u64()), Some(65));
    assert_eq!(streamdeck.get_brightness(), 65, "StreamDeckEngine brightness musi zostać zaktualizowane do 65");

    // Cleanup
    shutdown_signal.store(true, Ordering::SeqCst);
    let _ = server_task.await;
}

#[test]
fn test_workspace_scripts_and_tasks_generation() {
    use algodeck::workspace::WorkspaceManager;
    let ws_path = std::env::temp_dir().join(format!("algodeck_test_ws_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&ws_path);
    let _ = std::fs::create_dir_all(&ws_path);

    WorkspaceManager::ensure_problem_scripts(&ws_path, "chw", "Chwasty", 1.0);

    let p_dir = ws_path.join("chw");
    let algo_dir = p_dir.join(".algo");
    assert!(algo_dir.join("run.sh").exists(), "run.sh musi istnieć");
    assert!(algo_dir.join("test.sh").exists(), "test.sh musi istnieć");
    assert!(algo_dir.join("kill.sh").exists(), "kill.sh musi istnieć");
    assert!(algo_dir.join("accept.sh").exists(), "accept.sh musi istnieć");
    assert!(p_dir.join(".vscode/tasks.json").exists(), "problem tasks.json musi istnieć");
    assert!(ws_path.join(".vscode/tasks.json").exists(), "root workspace tasks.json musi istnieć");

    let run_content = std::fs::read_to_string(algo_dir.join("run.sh")).unwrap();
    assert!(run_content.contains("chw.cpp"));
    assert!(run_content.contains("-std=c++20"));

    let test_content = std::fs::read_to_string(algo_dir.join("test.sh")).unwrap();
    assert!(test_content.contains("AlgoDeck Test Runner"));
    assert!(test_content.contains("CHW"));

    let _ = std::fs::remove_dir_all(&ws_path);
}


