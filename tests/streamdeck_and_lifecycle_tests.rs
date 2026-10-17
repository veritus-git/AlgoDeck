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
    assert!(algo_dir.join("run.cmd").exists(), "run.cmd musi istnieć dla Windows");
    assert!(algo_dir.join("test.sh").exists(), "test.sh musi istnieć");
    assert!(algo_dir.join("test.cmd").exists(), "test.cmd musi istnieć dla Windows");
    assert!(algo_dir.join("kill.sh").exists(), "kill.sh musi istnieć");
    assert!(algo_dir.join("kill.cmd").exists(), "kill.cmd musi istnieć dla Windows");
    assert!(algo_dir.join("accept.sh").exists(), "accept.sh musi istnieć");
    assert!(p_dir.join(".vscode/tasks.json").exists(), "problem tasks.json musi istnieć");
    assert!(ws_path.join(".vscode/tasks.json").exists(), "root workspace tasks.json musi istnieć");

    let run_content = std::fs::read_to_string(algo_dir.join("run.sh")).unwrap();
    assert!(run_content.contains("chw.cpp"));
    assert!(run_content.contains("-std=c++20"));

    let test_content = std::fs::read_to_string(algo_dir.join("test.sh")).unwrap();
    assert!(test_content.contains("AlgoDeck Test Runner"));
    assert!(test_content.contains("CHW"));

    let p_tasks = std::fs::read_to_string(p_dir.join(".vscode/tasks.json")).unwrap();
    assert!(p_tasks.contains("${workspaceFolder}/.algo/run.sh"));
    assert!(p_tasks.contains("${workspaceFolder}/$TASK/.algo/run.sh"));
    assert!(p_tasks.contains("run.cmd"), "tasks.json musi zawierać wsparcie run.cmd dla Windows");

    let root_tasks = std::fs::read_to_string(ws_path.join(".vscode/tasks.json")).unwrap();
    assert!(root_tasks.contains("${workspaceFolder}/.algo/run.sh"));
    assert!(root_tasks.contains("${workspaceFolder}/$TASK/.algo/run.sh"));
    assert!(root_tasks.contains("run.cmd"), "root tasks.json musi zawierać wsparcie run.cmd dla Windows");

    let _ = std::fs::remove_dir_all(&ws_path);
}

#[test]
fn test_manual_problem_creation_and_execution() {
    use algodeck::pdf::{ProblemAnalysis, TestCase};
    use algodeck::workspace::WorkspaceManager;

    let ws_path = std::env::temp_dir().join(format!("algodeck_test_manual_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&ws_path);
    let _ = std::fs::create_dir_all(&ws_path);

    let analysis = ProblemAnalysis {
        problem_id: "sum".to_string(),
        title: "Suma Liczb".to_string(),
        time_limit_sec: 1.0,
        memory_limit_mb: 128,
        tests: vec![
            TestCase {
                id: "test_1".to_string(),
                name: "Przykład 1".to_string(),
                input: "2 3\n".to_string(),
                expected_output: "5\n".to_string(),
                is_edge_case: false,
                tag: "[RĘCZNY]".to_string(),
            },
        ],
        used_engine: "Ręczny kreator zadania".to_string(),
    };

    let p_dir = WorkspaceManager::create_problem(&ws_path, &analysis, None).expect("Utworzenie zadania ręcznego musi się udać");
    assert!(p_dir.exists());

    let test_in = p_dir.join(".algo/tests/test_1.in");
    let test_out = p_dir.join(".algo/tests/test_1.out");
    assert!(test_in.exists(), "test_1.in musi zostać utworzony");
    assert!(test_out.exists(), "test_1.out musi zostać utworzony");
    assert_eq!(std::fs::read_to_string(test_in).unwrap().trim(), "2 3");
    assert_eq!(std::fs::read_to_string(test_out).unwrap().trim(), "5");

    let cpp_file = p_dir.join("sum.cpp");
    assert!(cpp_file.exists());
    std::fs::write(&cpp_file, r#"#include <iostream>
using namespace std;
int main() {
    int a, b;
    if (cin >> a >> b) {
        cout << (a + b) << endl;
    }
    return 0;
}
"#).unwrap();

    let settings = AppSettings {
        workspace_dir: ws_path.to_string_lossy().to_string(),
        ..AppSettings::default()
    };
    let runner = TestExecutor::new(Arc::new(RwLock::new(settings)));
    let summary = runner.run_all_tests("sum");
    assert!(summary.success, "Kompilacja i wykonanie testów musi się udać");
    assert_eq!(summary.total_tests, 1);
    assert_eq!(summary.passed_count, 1);
    assert!(summary.all_passed);

    let deck = StreamDeckEngine::new();
    deck.set_active_problem_with_meta("sum", 1, 1);
    let state = deck.get_state();
    assert_eq!(state.active_problem_id.as_deref(), Some("sum"));

    let _ = std::fs::remove_dir_all(&ws_path);
}

#[tokio::test]
async fn test_vscode_detection_and_multi_socket_bridge() {
    // 1. Sprawdź gdy puste ID
    assert!(!algodeck::server::is_vscode_open_for_problem(""));

    // 2. Symuluj UNIX domain socket utworzony przez algodeckworkspace
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixListener;
        use std::io::{Read, Write};

        let sock_path = "/tmp/algodeck_vscode_algodeckworkspace.sock";
        let _ = std::fs::remove_file(sock_path);

        if let Ok(listener) = UnixListener::bind(sock_path) {
            let handle = std::thread::spawn(move || {
                for _ in 0..3 {
                    if let Ok((mut stream, _)) = listener.accept() {
                        let mut buf = [0u8; 128];
                        let _ = stream.read(&mut buf);
                        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\nOK");
                    }
                }
            });

            // is_vscode_open_for_problem powinien wykryć algodeckworkspace.sock
            assert!(algodeck::server::is_vscode_open_for_problem("kol"));

            // send_bridge_request również powinien przesłać zapytanie
            let sent = algodeck::server::send_bridge_request("test", "kol", "/tmp/pdir");
            assert!(sent, "send_bridge_request musi nawiązać połączenie przez socket");

            // send_bridge_open powinien natychmiast przekazać polecenie otwarcia pliku
            let opened = algodeck::server::send_bridge_open("kol", "/tmp/pdir", "/tmp/pdir/kol.cpp", "auto");
            assert!(opened, "send_bridge_open musi nawiązać połączenie przez socket i otworzyć plik");

            let _ = handle.join();
            let _ = std::fs::remove_file(sock_path);
        }
    }
}

#[test]
fn test_download_registry_deduplication() {
    use algodeck::config::DownloadedPdfRegistry;

    let tmp_dir = std::env::temp_dir().join("algodeck_test_registry_dedup");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    let _ = std::fs::create_dir_all(&tmp_dir);

    let pdf_a = tmp_dir.join("task_a.pdf");
    std::fs::write(&pdf_a, b"%PDF-1.4 test content for problem A").unwrap();

    let mut reg = DownloadedPdfRegistry {
        version: 1,
        updated_at: 0,
        entries: std::collections::HashMap::new(),
    };

    // 1. Pierwsza rejestracja pliku -> true (wykryto nowy plik)
    assert!(reg.check_and_register_if_new(&pdf_a), "Pierwsze pobranie musi być traktowane jako nowe");

    // 2. Kolejne sprawdzenie tego samego pliku -> false (nigdy więcej nie powiadamiamy)
    assert!(!reg.check_and_register_if_new(&pdf_a), "Ponowne zapytanie o ten sam plik musi zwrócić false");

    // 3. Sprawdzenie kopii pliku pod inną nazwą (identyczna zawartość / ten sam SHA256) -> false
    let pdf_a_copy = tmp_dir.join("task_a (1).pdf");
    std::fs::write(&pdf_a_copy, b"%PDF-1.4 test content for problem A").unwrap();
    assert!(!reg.check_and_register_if_new(&pdf_a_copy), "Duplikat treści pod inną nazwą musi zwrócić false");

    // 4. Prawdziwie nowy plik z inną zawartością -> true
    let pdf_b = tmp_dir.join("task_b.pdf");
    std::fs::write(&pdf_b, b"%PDF-1.4 DIFFERENT content for problem B").unwrap();
    assert!(reg.check_and_register_if_new(&pdf_b), "Nowe zadanie z innym hashem musi zostać zarejestrowane");

    // 5. Test czyszczenia rejestru: po wyczyszczeniu wpisów pliki znów mogą być traktowane jak świeże
    assert_eq!(reg.entries.len(), 2);
    assert!(reg.clear().is_ok());
    assert_eq!(reg.entries.len(), 0);
    assert!(reg.check_and_register_if_new(&pdf_a), "Po wyczyszczeniu rejestru plik A znów musi być traktowany jako nowy");

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_zip_olympiad_formats_and_large_packages() {
    use algodeck::workspace::WorkspaceManager;
    use std::io::Write;

    let tmp_ws = std::env::temp_dir().join(format!("algodeck_test_zip_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_ws);
    let _ = std::fs::create_dir_all(&tmp_ws);

    // Stwórz zadanie testowe
    let analysis = algodeck::pdf::ProblemAnalysis {
        problem_id: "sum".to_string(),
        title: "Suma Liczb".to_string(),
        time_limit_sec: 1.0,
        memory_limit_mb: 128,
        tests: Vec::new(),
        used_engine: "test".to_string(),
    };
    WorkspaceManager::create_problem(&tmp_ws, &analysis, None).expect("Stworzenie zadania");

    // Stwórz archiwum ZIP z formatami olimpijskimi: .we / .wy, .in / .out, podfoldery in/ i out/
    let mut zip_buf = Vec::new();
    {
        let mut zip_writer = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buf));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Test 1: format .in / .out
        zip_writer.start_file("1.in", options).unwrap();
        zip_writer.write_all(b"2 3\n").unwrap();
        zip_writer.start_file("1.out", options).unwrap();
        zip_writer.write_all(b"5\n").unwrap();

        // Test 2: format .we / .wy (standard Olimpiady Informatycznej)
        zip_writer.start_file("2a.we", options).unwrap();
        zip_writer.write_all(b"10 20\n").unwrap();
        zip_writer.start_file("2a.wy", options).unwrap();
        zip_writer.write_all(b"30\n").unwrap();

        // Test 3: format .in / .ans
        zip_writer.start_file("3.in", options).unwrap();
        zip_writer.write_all(b"100 200\n").unwrap();
        zip_writer.start_file("3.ans", options).unwrap();
        zip_writer.write_all(b"300\n").unwrap();

        // Test 4: podfoldery in/ oraz out/
        zip_writer.start_file("in/4.in", options).unwrap();
        zip_writer.write_all(b"7 8\n").unwrap();
        zip_writer.start_file("out/4.out", options).unwrap();
        zip_writer.write_all(b"15\n").unwrap();

        zip_writer.finish().unwrap();
    }

    let added = WorkspaceManager::add_tests_from_zip(&tmp_ws, "sum", &zip_buf).expect("Import ZIP");
    assert_eq!(added, 4, "Wszystkie 4 formaty testów olimpijskich (.in/.out, .we/.wy, .in/.ans, in/ / out/) muszą zostać poprawnie sparsowane!");

    let manifest = WorkspaceManager::get_manifest(&tmp_ws, "sum").expect("Manifest");
    assert_eq!(manifest.tests.len(), 4);

    let _ = std::fs::remove_dir_all(&tmp_ws);
}

#[tokio::test]
async fn test_large_file_upload_beyond_axum_default_limit() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let settings = Arc::new(RwLock::new(AppSettings::load()));
    let runner = Arc::new(TestExecutor::new(settings.clone()));
    let streamdeck = Arc::new(StreamDeckEngine::new());
    let (ws_sender, _) = broadcast::channel(10);
    let shutdown_signal = Arc::new(AtomicBool::new(false));

    let app_state = AppState {
        settings,
        runner,
        streamdeck,
        ws_sender,
        shutdown_signal,
    };
    let router = create_router(app_state);

    // Stwórz 5 MB payload (ponad domyślny 2 MB limit Axum)
    let boundary = "---------------------------974767299852498929531610575";
    let mut body_bytes = Vec::new();
    body_bytes.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body_bytes.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"large_test.zip\"\r\n");
    body_bytes.extend_from_slice(b"Content-Type: application/zip\r\n\r\n");
    body_bytes.resize(body_bytes.len() + 5 * 1024 * 1024, b'X');
    body_bytes.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    let req = Request::builder()
        .method("POST")
        .uri("/api/stage-file")
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .body(Body::from(body_bytes))
        .unwrap();

    let res = router.oneshot(req).await.unwrap();
    assert_ne!(res.status(), StatusCode::PAYLOAD_TOO_LARGE, "Axum nie może odrzucać plików większych niż domyślne 2 MB!");
    assert_eq!(res.status(), StatusCode::OK);
}





