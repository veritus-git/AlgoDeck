mod config;
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
#[command(about = "AlgoDeck ⚡ 100% Rust Olympic Programming Engine & Stream Deck Controller", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Uruchom demona (serwer Axum + sterownik Stream Deck)")]
    Daemon {
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    #[command(about = "Uruchom demona w tle jako niezależny proces")]
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
            // Domyślnie uruchamiamy demona
            run_daemon(8080);
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

fn check_streamdeck_conflicts() {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq StreamDeck.exe"])
            .output();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            if s.contains("StreamDeck.exe") {
                eprintln!("\x1b[1;33m[UWAGA] Wykryto działającą oficjalną aplikację Elgato (StreamDeck.exe)!");
                eprintln!("Aplikacja Elgato blokuje bezpośredni dostęp USB HID do urządzenia.");
                eprintln!("Zamknij StreamDeck.exe z paska zadań, aby AlgoDeck mógł natychmiast przejąć kontrolę nad Stream Deckiem.\x1b[0m\n");
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("pgrep")
            .args(["-i", "streamcontroller"])
            .output();
        if let Ok(out) = output {
            if !out.stdout.is_empty() {
                eprintln!("\x1b[1;33m[UWAGA] Wykryto działający proces StreamController!");
                eprintln!("StreamController może blokować bezpośredni dostęp do urządzenia.");
                eprintln!("Zamknij StreamController, aby AlgoDeck mógł sterować klawiszami.\x1b[0m\n");
            }
        }
    }
}

fn run_daemon(port: u16) {
    println!("{}", r#"
  █████╗ ██╗      ██████╗  ██████╗ ██████╗ ███████╗ ██████╗██╗  ██╗
 ██╔══██╗██║     ██╔════╝ ██╔═══██╗██╔══██╗██╔════╝██╔════╝██║ ██╔╝
 ███████║██║     ██║  ███╗██║   ██║██║  ██║█████╗  ██║     █████═╝ 
 ██╔══██║██║     ██║   ██║██║   ██║██║  ██║██╔══╝  ██║     ██╔═██╗ 
 ██║  ██║███████╗╚██████╔╝╚██████╔╝██████╔╝███████╗╚██████╗██║  ██╗
 ╚═╝  ╚═╝╚══════╝ ╚═════╝  ╚═════╝ ╚═════╝ ╚══════╝ ╚═════╝╚═╝  ╚═╝

       ⚡  K O Ł O   M A P   -   O L I M P I C K I E   C + +  ⚡
       ⚡       100% RUST ENGINE  |  STREAM DECK HID        ⚡
"#);
    println!("⚡ AlgoDeck Daemon v2.0.0 (100% Rust Architecture) na porcie {}", port);
    println!("--------------------------------------------------");

    check_streamdeck_conflicts();

    let settings = AppSettings::load();
    let runner = Arc::new(TestExecutor::new(settings.clone()));
    let streamdeck = Arc::new(StreamDeckEngine::new());
    let (ws_sender, _) = broadcast::channel(100);
    let shutdown_signal = Arc::new(AtomicBool::new(false));

    // Callback na fizyczne przyciski Stream Decka
    let runner_clone = runner.clone();
    let streamdeck_clone = streamdeck.clone();
    let settings_clone = settings.clone();

    fn open_in_vscode(settings: &config::AppSettings, problem_id: &str) {
        if problem_id.is_empty() {
            return;
        }
        let ws_dir = settings.get_workspace_path();
        let p_dir = workspace::WorkspaceManager::get_problem_dir(&ws_dir, problem_id);
        let sol_file = p_dir.join(format!("{}.cpp", problem_id));
        let pdf_file = p_dir.join(format!("{}.pdf", problem_id));
        let pdf_fallback = p_dir.join(".algo").join("statement.pdf");
        let actual_pdf = if pdf_file.exists() {
            Some(pdf_file)
        } else if pdf_fallback.exists() {
            Some(pdf_fallback)
        } else {
            None
        };

        let temp_task_file = std::env::temp_dir().join("algodeck_active_task.txt");
        let _ = std::fs::write(&temp_task_file, problem_id.to_uppercase());

        #[cfg(target_os = "windows")]
        {
            let mut args = vec!["--reuse-window".to_string()];
            if settings.vscode_mode == "separate_windows" {
                args.push(p_dir.to_string_lossy().to_string());
            } else {
                args.push(ws_dir.to_string_lossy().to_string());
            }
            if sol_file.exists() {
                args.push(sol_file.to_string_lossy().to_string());
            }
            if settings.open_pdf_in_vscode {
                if let Some(ref p) = actual_pdf {
                    args.push(p.to_string_lossy().to_string());
                }
            }

            let win_code_cmd = std::env::var("LOCALAPPDATA")
                .map(|l| std::path::PathBuf::from(l).join("Programs/Microsoft VS Code/bin/code.cmd"))
                .ok();

            if let Some(ref code_path) = win_code_cmd {
                if code_path.exists() {
                    let _ = std::process::Command::new(code_path).args(&args).spawn();
                    return;
                }
            }

            let mut cmd = std::process::Command::new("cmd");
            cmd.arg("/C").arg("code");
            for a in args {
                cmd.arg(a);
            }
            let _ = cmd.spawn();
        }

        #[cfg(not(target_os = "windows"))]
        {
            let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into());
            let xauthority = std::env::var("XAUTHORITY").unwrap_or_else(|_| "/home/linux/.Xauthority".into());

            // 1. Skrypt sd_algo_code.sh jeśli obecny
            let sd_code = dirs::home_dir().map(|h| h.join(".local/bin/sd_algo_code.sh"));
            let mut script_launched = false;
            if let Some(ref script) = sd_code {
                if script.exists() {
                    let res = std::process::Command::new(script)
                        .arg(&p_dir)
                        .arg(&sol_file)
                        .env("DISPLAY", &display)
                        .env("XAUTHORITY", &xauthority)
                        .spawn();
                    if res.is_ok() {
                        script_launched = true;
                    }
                }
            }

            if !script_launched {
                let code_bin = if std::path::Path::new("/usr/bin/code").exists() {
                    "/usr/bin/code"
                } else {
                    "code"
                };
                let mut cmd = std::process::Command::new(code_bin);
                cmd.arg("--reuse-window");
                if settings.vscode_mode == "separate_windows" {
                    cmd.arg(&p_dir);
                } else {
                    cmd.arg(&ws_dir);
                }
                if sol_file.exists() {
                    cmd.arg(&sol_file);
                }
                if settings.open_pdf_in_vscode {
                    if let Some(ref p) = actual_pdf {
                        cmd.arg(p);
                    }
                }
                cmd.env("DISPLAY", &display);
                cmd.env("XAUTHORITY", &xauthority);
                let _ = cmd.spawn();
            }

            let _ = std::process::Command::new("notify-send")
                .args(["AlgoDeck", &format!("Otwarto zadanie {} w Visual Studio Code", problem_id.to_uppercase())])
                .env("DISPLAY", &display)
                .env("XAUTHORITY", &xauthority)
                .spawn();
        }
    }

    let action_callback = Arc::new(move |action: String, _key_idx: usize| {
        let active_id = streamdeck_clone.get_state().active_problem_id.unwrap_or_default();
        let ws_dir = settings_clone.get_workspace_path();
        let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into());
        let xauthority = std::env::var("XAUTHORITY").unwrap_or_else(|_| "/home/linux/.Xauthority".into());

        if action.starts_with("select_task:") {
            let pid = action.trim_start_matches("select_task:").to_string();
            let problems = workspace::WorkspaceManager::list_problems(&ws_dir);
            let cur_idx = problems.iter().position(|p| p.problem_id == pid).unwrap_or(0) + 1;
            streamdeck_clone.set_active_problem_with_meta(&pid, cur_idx, problems.len().max(1));
            open_in_vscode(&settings_clone, &pid);
            return;
        }

        if action.starts_with("menu_page:") {
            if let Ok(page) = action.trim_start_matches("menu_page:").parse::<usize>() {
                let problems = workspace::WorkspaceManager::list_problems(&ws_dir);
                let pids: Vec<String> = problems.into_iter().map(|p| p.problem_id).collect();
                streamdeck_clone.set_menu_screen(&pids, page);
                return;
            }
        }

        match action.as_str() {
            "open_vscode" => {
                if !active_id.is_empty() {
                    open_in_vscode(&settings_clone, &active_id);
                }
            }
            "run_all" => {
                if !active_id.is_empty() {
                    let sd_test = dirs::home_dir().map(|h| h.join(".local/bin/sd_algo_test_vscode.sh"));
                    let p_dir = workspace::WorkspaceManager::get_problem_dir(&ws_dir, &active_id);
                    if let Some(ref s) = sd_test {
                        if s.exists() {
                            let _ = std::process::Command::new(s)
                                .arg(&p_dir)
                                .arg(&active_id)
                                .env("DISPLAY", &display)
                                .env("XAUTHORITY", &xauthority)
                                .spawn();
                            return;
                        }
                    }

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
                    let sd_run = dirs::home_dir().map(|h| h.join(".local/bin/sd_algo_run_vscode.sh"));
                    let p_dir = workspace::WorkspaceManager::get_problem_dir(&ws_dir, &active_id);
                    if let Some(ref s) = sd_run {
                        if s.exists() {
                            let _ = std::process::Command::new(s)
                                .arg(&p_dir)
                                .arg(&active_id)
                                .env("DISPLAY", &display)
                                .env("XAUTHORITY", &xauthority)
                                .spawn();
                            return;
                        }
                    }

                    let mut cmd = std::process::Command::new("x-terminal-emulator");
                    cmd.args(["-e", &format!("algodeck run {}", active_id)]);
                    cmd.env("DISPLAY", &display);
                    cmd.env("XAUTHORITY", &xauthority);
                    let _ = cmd.spawn();
                }
            }
            "new_task" => {
                let self_exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("/home/linux/.local/bin/algodeck"));
                let mut cmd = std::process::Command::new(self_exe);
                cmd.arg("--tab=new");
                cmd.env("DISPLAY", &display);
                cmd.env("XAUTHORITY", &xauthority);
                let _ = cmd.spawn();
            }
            "open_settings" => {
                let self_exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("/home/linux/.local/bin/algodeck"));
                let mut cmd = std::process::Command::new(self_exe);
                cmd.arg("--tab=settings");
                cmd.env("DISPLAY", &display);
                cmd.env("XAUTHORITY", &xauthority);
                let _ = cmd.spawn();
            }
            "open_menu" => {
                let problems = workspace::WorkspaceManager::list_problems(&ws_dir);
                let pids: Vec<String> = problems.into_iter().map(|p| p.problem_id).collect();
                streamdeck_clone.set_menu_screen(&pids, 0);
            }
            "back_to_task" => {
                if !active_id.is_empty() {
                    let problems = workspace::WorkspaceManager::list_problems(&ws_dir);
                    let cur_idx = problems.iter().position(|p| p.problem_id == active_id).unwrap_or(0) + 1;
                    streamdeck_clone.set_active_problem_with_meta(&active_id, cur_idx, problems.len().max(1));
                } else {
                    streamdeck_clone.set_idle_screen();
                }
            }
            "exit_to_idle" => {
                streamdeck_clone.set_idle_screen();
            }
            "start_or_resume" => {
                let problems = workspace::WorkspaceManager::list_problems(&ws_dir);
                if let Some(first) = problems.first() {
                    streamdeck_clone.set_active_problem_with_meta(&first.problem_id, 1, problems.len().max(1));
                    open_in_vscode(&settings_clone, &first.problem_id);
                } else {
                    let self_exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("/home/linux/.local/bin/algodeck"));
                    let mut cmd = std::process::Command::new(self_exe);
                    cmd.arg("--tab=new");
                    cmd.env("DISPLAY", &display);
                    cmd.env("XAUTHORITY", &xauthority);
                    let _ = cmd.spawn();
                }
            }
            "next_task" | "prev_task" => {
                let problems = workspace::WorkspaceManager::list_problems(&ws_dir);
                if !problems.is_empty() {
                    let cur_idx = problems.iter().position(|p| p.problem_id == active_id).unwrap_or(0);
                    let new_idx = if action == "next_task" {
                        (cur_idx + 1) % problems.len()
                    } else {
                        (cur_idx + problems.len() - 1) % problems.len()
                    };
                    let new_pid = &problems[new_idx].problem_id;
                    streamdeck_clone.set_active_problem_with_meta(new_pid, new_idx + 1, problems.len());
                    open_in_vscode(&settings_clone, new_pid);
                }
            }
            "kill_process" => {
                if !active_id.is_empty() {
                    let p_dir = workspace::WorkspaceManager::get_problem_dir(&ws_dir, &active_id);
                    let kill_script = p_dir.join(".algo").join("kill.sh");
                    if kill_script.exists() {
                        let _ = std::process::Command::new("bash").arg(kill_script).spawn();
                    }
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
    if !problems.is_empty() {
        let mut target_idx = 0;
        if let Ok(saved) = std::fs::read_to_string("/tmp/algodeck_active_task.txt") {
            let saved_clean = saved.trim().to_lowercase();
            if let Some(pos) = problems.iter().position(|p| p.problem_id == saved_clean) {
                target_idx = pos;
            }
        }
        let prob = &problems[target_idx];
        streamdeck.set_active_problem_with_meta(&prob.problem_id, target_idx + 1, problems.len());
    } else {
        streamdeck.set_idle_screen();
    }

    // Uruchom wątek sprzętowy Stream Deck USB HID
    streamdeck.clone().start_hardware_thread(action_callback);

    let streamdeck_shutdown = streamdeck.clone();
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
                streamdeck_shutdown.stop();
            }
        }
    });

    // Zawsze wygaszamy ekran fizycznego kontrolera przy wyjściu
    StreamDeckEngine::blank_device_if_connected();
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
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill").args(["/F", "/IM", "algodeck.exe"]).output();
    }
    std::thread::sleep(std::time::Duration::from_millis(150));
    // Bezpośrednie wyczyszczenie ekranu Stream Decka przez USB HID
    StreamDeckEngine::blank_device_if_connected();
    println!("✓ AlgoDeck zatrzymany, ekran Stream Decka wygaszony.");
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
