mod config;
mod gui;
mod native_host;
mod pdf;
mod runner;
mod server;
mod streamdeck;
mod workspace;

use clap::{Parser, Subcommand};
use config::AppSettings;
use runner::TestExecutor;
use streamdeck::StreamDeckEngine;
use workspace::WorkspaceManager;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

#[derive(Parser)]
#[command(name = "algodeck")]
#[command(author = "AlgoDeck Team")]
#[command(version = "2.0.0")]
#[command(about = "AlgoDeck ⚡ 100% Rust Olympic Programming Environment & Stream Deck Controller", long_about = None)]
struct Cli {
    #[arg(short, long, help = "Plik PDF z treścią zadania do otwarcia w GUI")]
    pdf: Option<String>,

    #[arg(long, help = "Uruchom tylko ikonę w zasobniku systemowym (tray)")]
    tray: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Uruchom graficzny panel AlgoDeck (domyślne)")]
    Gui {
        #[arg(short, long)]
        pdf: Option<String>,
        #[arg(long)]
        tray: bool,
    },

    #[command(about = "Uruchom daemona w tle (serwer Axum + sterownik Stream Deck)")]
    Daemon {
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    #[command(about = "Uruchom daemona w tle jako niezależny proces")]
    Start,

    #[command(about = "Zatrzymaj działający proces AlgoDeck")]
    Stop,

    #[command(about = "Sprawdź status działania AlgoDeck")]
    Status,

    #[command(about = "Mostek Native Messaging dla rozszerzenia przeglądarki Chrome/Brave")]
    NativeHost,

    #[command(about = "Skompiluj i uruchom testy dla zadania w konsoli")]
    Test {
        problem_id: String,
    },

    #[command(about = "Uruchom rozwiązanie C++ interaktywnie w konsoli")]
    Run {
        problem_id: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // Domyślnie uruchamiamy okno GUI
            if let Err(e) = gui::run_gui(cli.pdf, cli.tray) {
                eprintln!("Błąd uruchomienia GUI: {:?}", e);
            }
        }
        Some(Commands::Gui { pdf, tray }) => {
            let initial_pdf = pdf.or(cli.pdf);
            let tray_mode = tray || cli.tray;
            if let Err(e) = gui::run_gui(initial_pdf, tray_mode) {
                eprintln!("Błąd uruchomienia GUI: {:?}", e);
            }
        }
        Some(Commands::Daemon { port }) => {
            run_daemon(port);
        }
        Some(Commands::Start) => {
            start_detached_daemon();
        }
        Some(Commands::Stop) => {
            stop_daemon();
        }
        Some(Commands::Status) => {
            check_status();
        }
        Some(Commands::NativeHost) => {
            native_host::run_native_host();
        }
        Some(Commands::Test { problem_id }) => {
            run_cli_test(&problem_id);
        }
        Some(Commands::Run { problem_id }) => {
            run_cli_interactive(&problem_id);
        }
    }
}

fn run_daemon(port: u16) {
    println!("⚡ AlgoDeck Daemon v2.0.0 (100% Rust Architecture)");
    println!("--------------------------------------------------");

    let settings = AppSettings::load();
    let runner = Arc::new(TestExecutor::new(settings.clone()));
    let streamdeck = Arc::new(StreamDeckEngine::new());
    let (ws_sender, _) = broadcast::channel(100);
    let shutdown_signal = Arc::new(AtomicBool::new(false));

    // Callback na fizyczne przyciski Stream Decka
    let runner_clone = runner.clone();
    let streamdeck_clone = streamdeck.clone();
    let settings_clone = settings.clone();

    let action_callback = Arc::new(move |action: String, _key_idx: usize| {
        let active_id = streamdeck_clone.get_state().active_problem_id.unwrap_or_default();
        match action.as_str() {
            "open_vscode" => {
                if !active_id.is_empty() {
                    let _ = std::process::Command::new("code")
                        .arg("--reuse-window")
                        .arg(&settings_clone.workspace_dir)
                        .arg(format!("{}/{}/{}.cpp", settings_clone.workspace_dir, active_id, active_id))
                        .spawn();
                }
            }
            "run_all" => {
                if !active_id.is_empty() {
                    let r = runner_clone.run_all_tests(&active_id);
                    if r.all_passed {
                        streamdeck_clone.update_key_status(6, "OK", Some(&format!("{:.0}ms", r.total_time_ms)));
                    } else {
                        streamdeck_clone.update_key_status(6, "WA", Some(&format!("{}/{}", r.passed_count, r.total_tests)));
                    }
                }
            }
            "run_interactive" => {
                if !active_id.is_empty() {
                    let _ = std::process::Command::new("x-terminal-emulator")
                        .args(["-e", &format!("algodeck run {}", active_id)])
                        .spawn();
                }
            }
            "next_task" | "prev_task" => {
                let ws_path = settings_clone.get_workspace_path();
                let problems = workspace::WorkspaceManager::list_problems(&ws_path);
                if !problems.is_empty() {
                    let cur_idx = problems.iter().position(|p| p.problem_id == active_id).unwrap_or(0);
                    let new_idx = if action == "next_task" {
                        (cur_idx + 1) % problems.len()
                    } else {
                        (cur_idx + problems.len() - 1) % problems.len()
                    };
                    streamdeck_clone.set_active_problem(&problems[new_idx].problem_id);
                }
            }
            "open_menu" => {
                let self_exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("algodeck"));
                let _ = std::process::Command::new(self_exe).spawn();
            }
            "kill_process" => {
                if !active_id.is_empty() {
                    let _ = std::process::Command::new("pkill").arg("-f").arg(&active_id).output();
                    streamdeck_clone.update_key_status(8, "DANGER", Some("KILLED"));
                }
            }
            _ => {}
        }
    });

    // Ustaw początkowe aktywne zadanie na Stream Decku jeśli istnieje w workspace
    let ws_path = settings.get_workspace_path();
    let problems = workspace::WorkspaceManager::list_problems(&ws_path);
    if let Some(first) = problems.first() {
        streamdeck.set_active_problem(&first.problem_id);
    } else {
        streamdeck.set_idle_screen();
    }

    // Uruchom wątek sprzętowy Stream Deck USB HID
    streamdeck.clone().start_hardware_thread(action_callback);

    let app_state = server::AppState {
        settings: Arc::new(RwLock::new(settings)),
        streamdeck,
        runner,
        ws_sender,
        shutdown_signal: shutdown_signal.clone(),
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let router = server::create_router(app_state);
        let addr = format!("127.0.0.1:{}", port);
        println!("🚀 Serwer HTTP i WebSocket aktywny pod adresem: http://{}", addr);

        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Błąd bindowania portu {}: {}", port, e);
                return;
            }
        };

        let server_future = axum::serve(listener, router);

        tokio::select! {
            _ = server_future => {},
            _ = tokio::signal::ctrl_c() => {
                println!("\n⏹ Zatrzymywanie daemona AlgoDeck (Ctrl+C)...");
            }
        }
    });
}

fn start_detached_daemon() {
    match ureq::get("http://127.0.0.1:8080/api/status").timeout(std::time::Duration::from_millis(500)).call() {
        Ok(_) => {
            println!("⚡ AlgoDeck już działa w tle na porcie 8080.");
            return;
        }
        Err(_) => {}
    }

    #[cfg(target_os = "linux")]
    {
        let status = std::process::Command::new("systemctl")
            .args(["--user", "start", "algodeck.service"])
            .status();
        if let Ok(s) = status {
            if s.success() {
                std::thread::sleep(std::time::Duration::from_millis(300));
                if ureq::get("http://127.0.0.1:8080/api/status").timeout(std::time::Duration::from_millis(500)).call().is_ok() {
                    println!("✓ Usługa algodeck.service została uruchomiona przez systemd!");
                    return;
                }
            }
        }
    }

    println!("⚡ Uruchamianie daemona AlgoDeck w tle...");
    let self_exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("algodeck"));
    match std::process::Command::new(self_exe).arg("daemon").spawn() {
        Ok(_) => {
            println!("✓ Daemon został pomyślnie uruchomiony w tle!");
        }
        Err(e) => {
            eprintln!("Błąd startu daemona: {}", e);
        }
    }
}

fn stop_daemon() {
    println!("⏹ Zatrzymywanie AlgoDeck...");
    let _ = ureq::post("http://127.0.0.1:8080/api/shutdown").timeout(std::time::Duration::from_millis(800)).call();
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("systemctl").args(["--user", "stop", "algodeck.service"]).output();
        let _ = std::process::Command::new("pkill").arg("-f").arg("algodeck daemon").output();
        let _ = std::process::Command::new("pkill").arg("-f").arg("algodeck-gui").output();
    }
    println!("✓ AlgoDeck zatrzymany.");
}

fn check_status() {
    match ureq::get("http://127.0.0.1:8080/api/status").timeout(std::time::Duration::from_millis(600)).call() {
        Ok(res) => {
            if let Ok(json) = res.into_json::<serde_json::Value>() {
                println!("⚡ AlgoDeck Status: ONLINE");
                println!("• Wersja: {}", json.get("version").and_then(|v| v.as_str()).unwrap_or("unknown"));
                println!("• Aktywne zadanie: {}", json.get("active_problem").and_then(|v| v.as_str()).unwrap_or("brak"));
                println!("• Workspace: {}", json.get("workspace_dir").and_then(|v| v.as_str()).unwrap_or(""));
                let sd = json.get("streamdeck_connected").and_then(|v| v.as_bool()).unwrap_or(false);
                println!("• Stream Deck USB: {}", if sd { "POŁĄCZONY" } else { "Wirtualny (Brak USB)" });
            }
        }
        Err(_) => {
            println!("⚡ AlgoDeck Status: OFFLINE (Daemon nie jest uruchomiony)");
        }
    }
}

fn run_cli_test(problem_id: &str) {
    let settings = AppSettings::load();
    let runner = TestExecutor::new(settings);
    println!("⚗ Uruchamianie testów dla zadania: {}", problem_id.to_uppercase());

    let summary = runner.run_all_tests(problem_id);
    for res in &summary.results {
        let symbol = if res.verdict == "OK" { "✓" } else { "✗" };
        println!("  {} [{}] {} - {:.1}ms", symbol, res.verdict, res.test_id, res.execution_time_ms);
        if res.verdict != "OK" && !res.diff.is_empty() {
            println!("     {}", res.diff.replace('\n', "\n     "));
        }
    }

    if summary.all_passed {
        println!("\n✅ WSZYSTKIE TESTY ZALICZONE! ({}/{} w {:.1}ms)", summary.passed_count, summary.total_tests, summary.total_time_ms);
    } else {
        println!("\n❌ NIEKTÓRE TESTY NIE POWIODŁY SIĘ! ({}/{} zaliczone)", summary.passed_count, summary.total_tests);
    }
}

fn run_cli_interactive(problem_id: &str) {
    let settings = AppSettings::load();
    let runner = TestExecutor::new(settings.clone());
    let comp = runner.compile(problem_id, false);
    if !comp.success {
        eprintln!("Błąd kompilacji:\n{}", comp.error);
        return;
    }

    let p_dir = WorkspaceManager::get_problem_dir(&settings.get_workspace_path(), problem_id);
    let bin = p_dir.join(problem_id);
    println!("▶ Uruchamianie {} (wprowadź dane wejściowe, zakończ Ctrl+D):\n", problem_id);
    let _ = std::process::Command::new(bin).status();
}
