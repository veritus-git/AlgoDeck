use crate::config::AppSettings;
use crate::pdf::{PdfParser, ProblemAnalysis, TestCase};
use crate::runner::{CompileResult, RunAllSummary, TestExecutor, TestResult};
use crate::streamdeck::{StreamDeckEngine, StreamDeckState};
use crate::workspace::{ProblemManifest, WorkspaceManager};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Multipart, Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{delete, get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct AppState {
    pub settings: Arc<std::sync::RwLock<AppSettings>>,
    pub streamdeck: Arc<StreamDeckEngine>,
    pub runner: Arc<TestExecutor>,
    pub ws_sender: broadcast::Sender<String>,
    pub shutdown_signal: Arc<AtomicBool>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub version: String,
    pub active_problem: Option<String>,
    pub workspace_dir: String,
    pub streamdeck_connected: bool,
    #[serde(default)]
    pub installed_at: u64,
}

#[derive(Deserialize)]
pub struct AutoImportRequest {
    pub file_path: String,
    #[serde(default)]
    pub auto_create: bool,
    pub problem_id: Option<String>,
    pub title: Option<String>,
    pub time_limit: Option<f64>,
    pub memory_limit: Option<u64>,
}

#[derive(Deserialize, Debug)]
pub struct ManualTaskRequest {
    pub problem_id: String,
    pub title: String,
    #[serde(default = "default_manual_time_limit")]
    pub time_limit: f64,
    #[serde(default = "default_manual_memory_limit")]
    pub memory_limit: u64,
    #[serde(default)]
    pub sample_cin: String,
    #[serde(default)]
    pub sample_cout: String,
    #[serde(default)]
    pub tests: Vec<ManualTestRequestItem>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ManualTestRequestItem {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub input: String,
    #[serde(default)]
    pub expected_output: String,
}

fn default_manual_time_limit() -> f64 {
    1.0
}
fn default_manual_memory_limit() -> u64 {
    128
}

#[derive(Deserialize, Default)]
pub struct ClearStagedRequest {
    pub file_type: Option<String>,
}

pub fn create_router(app_state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/status", get(get_status))
        .route("/initial-downloads", get(get_initial_downloads))
        .route("/settings", get(get_settings).post(post_settings))
        .route("/settings/open-gui", post(open_gui))
        .route("/problems", get(get_problems))
        .route("/problems/:id", delete(delete_problem))
        .route("/problem/:id", get(get_problem).delete(delete_problem))
        .route("/problems/:id/add-tests", post(add_tests))
        .route("/auto-import-pdf", post(auto_import_pdf))
        .route("/create-manual", post(create_manual))
        .route("/staged-state", get(get_staged_state))
        .route("/stage-file", post(stage_file))
        .route("/stage-by-path", post(stage_by_path))
        .route("/stage-path", post(stage_by_path))
        .route("/register-download", post(register_download))
        .route("/registered-downloads", get(get_registered_downloads))
        .route("/clear-download-registry", post(clear_download_registry))
        .route("/register-download-dirs", post(register_download_dirs))
        .route("/stage-refresh", post(stage_refresh))
        .route("/upload-staging", post(stage_file))
        .route("/clear-staged", post(clear_staged))
        .route("/start-staged", post(start_staged))
        .route("/create-from-staging", post(start_staged))
        .route("/set-active/:id", post(set_active))
        .route("/open-vscode/:id", post(open_vscode))
        .route("/switch-task/:direction", post(switch_task))
        .route("/compile/:id", post(compile_task))
        .route("/run-all/:id", post(run_all_task))
        .route("/run-test/:id/:test_id", post(run_single_test_task))
        .route("/kill/:id", post(kill_task))
        .route("/launch", post(launch_screen))
        .route("/exit", post(exit_screen))
        .route("/shutdown", post(shutdown_server))
        .route("/streamdeck/state", get(get_streamdeck_state))
        .route("/streamdeck/brightness", post(set_streamdeck_brightness))
        .route("/streamdeck/press/:key_id", post(press_streamdeck_key));

    let mut router = Router::new()
        .nest("/api", api_routes)
        .route("/settings", get(serve_settings_html))
        .route("/settings.css", get(serve_settings_css))
        .route("/settings.js", get(serve_settings_js))
        .route("/favicon.ico", get(serve_favicon))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // Serve static frontend if directory exists
    if let Some(frontend_dir) = find_frontend_dir() {
        router = router.fallback_service(ServeDir::new(frontend_dir));
    }

    router
}

fn find_frontend_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(dir) = std::env::var("ALGODECK_DIR") {
        candidates.push(PathBuf::from(&dir).join("frontend"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("frontend"));
            candidates.push(parent.join("../share/algodeck/frontend"));
            candidates.push(parent.join("../share/algodeck/extension"));
        }
    }
    candidates.push(PathBuf::from("frontend"));
    candidates.push(PathBuf::from("extension"));

    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            candidates.push(PathBuf::from(local).join("AlgoDeck/frontend"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join(".local/share/algodeck/frontend"));
            candidates.push(home.join(".local/share/algodeck/extension"));
        }
    }

    candidates.into_iter().find(|c| c.exists() && c.is_dir())
}

// ----------------- Handlers -----------------

#[derive(Deserialize)]
pub struct BrightnessRequest {
    pub brightness: u8,
}

async fn set_streamdeck_brightness(
    State(state): State<AppState>,
    Json(req): Json<BrightnessRequest>,
) -> Json<serde_json::Value> {
    state.streamdeck.set_brightness(req.brightness);
    {
        let mut s = state.settings.write().unwrap();
        s.streamdeck_brightness = req.brightness;
        let _ = s.save();
    }
    Json(serde_json::json!({ "success": true, "brightness": req.brightness }))
}

fn find_file_in_candidates(filename: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(dir) = std::env::var("ALGODECK_DIR") {
        candidates.push(PathBuf::from(&dir).join("extension").join(filename));
        candidates.push(PathBuf::from(&dir).join("frontend").join(filename));
    }
    if let Ok(dir) = std::env::var("ALGODECK_EXTENSION_DIR") {
        candidates.push(PathBuf::from(&dir).join(filename));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("extension").join(filename));
            candidates.push(parent.join("../extension").join(filename));
            candidates.push(parent.join("../share/algodeck/extension").join(filename));
            candidates.push(parent.join("../share/algodeck/frontend").join(filename));
            candidates.push(parent.join("frontend").join(filename));
        }
    }

    candidates.push(PathBuf::from("extension").join(filename));
    candidates.push(PathBuf::from("frontend").join(filename));

    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&local).join("AlgoDeck/extension").join(filename));
            candidates.push(PathBuf::from(&local).join("AlgoDeck/frontend").join(filename));
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            candidates.push(PathBuf::from(appdata).join("AlgoDeck/extension").join(filename));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join(".local/share/algodeck/extension").join(filename));
            candidates.push(home.join(".local/share/algodeck/frontend").join(filename));
        }
    }

    for c in candidates {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

async fn serve_settings_html() -> impl IntoResponse {
    if let Some(p) = find_file_in_candidates("settings.html") {
        if let Ok(content) = fs::read_to_string(p) {
            return axum::response::Html(content).into_response();
        }
    }
    (StatusCode::NOT_FOUND, "Settings page not found").into_response()
}

async fn serve_settings_css() -> impl IntoResponse {
    if let Some(p) = find_file_in_candidates("settings.css") {
        if let Ok(content) = fs::read_to_string(p) {
            return ([(axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8")], content).into_response();
        }
    }
    (StatusCode::NOT_FOUND, "CSS not found").into_response()
}

async fn serve_settings_js() -> impl IntoResponse {
    if let Some(p) = find_file_in_candidates("settings.js") {
        if let Ok(content) = fs::read_to_string(p) {
            return ([(axum::http::header::CONTENT_TYPE, "application/javascript; charset=utf-8")], content).into_response();
        }
    }
    (StatusCode::NOT_FOUND, "JS not found").into_response()
}

async fn serve_favicon() -> impl IntoResponse {
    if let Some(p) = find_file_in_candidates("favicon.ico")
        .or_else(|| find_file_in_candidates("icons/icon48.png"))
        .or_else(|| find_file_in_candidates("icon48.png"))
    {
        if let Ok(bytes) = fs::read(p) {
            return ([(axum::http::header::CONTENT_TYPE, "image/png")], bytes).into_response();
        }
    }
    (StatusCode::NOT_FOUND, "Favicon not found").into_response()
}

async fn get_status(State(state): State<AppState>) -> Json<StatusResponse> {
    let settings = state.settings.read().unwrap().clone();
    let sd_state = state.streamdeck.get_state();
    let initial = crate::config::InitialDownloads::load_or_init();
    Json(StatusResponse {
        status: "ok".to_string(),
        version: "2.0.0".to_string(),
        active_problem: sd_state.active_problem_id,
        workspace_dir: settings.workspace_dir,
        streamdeck_connected: sd_state.is_hardware_connected,
        installed_at: initial.installed_at,
    })
}

async fn get_initial_downloads() -> Json<crate::config::InitialDownloads> {
    let initial = crate::config::InitialDownloads::load_or_init();
    Json(initial)
}

async fn get_settings(State(state): State<AppState>) -> Json<AppSettings> {
    let settings = state.settings.read().unwrap().clone();
    Json(settings)
}

async fn post_settings(
    State(state): State<AppState>,
    Json(new_settings): Json<AppSettings>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state.streamdeck.set_brightness(new_settings.streamdeck_brightness);
    apply_autostart_setting(new_settings.autostart_on_boot);
    let ws_path = new_settings.get_workspace_path();
    apply_macos_vscode_window_setting(&new_settings.macos_window_mode, Some(&ws_path));
    let _ = new_settings.save();
    {
        let mut w = state.settings.write().unwrap();
        *w = new_settings;
    }
    Ok(Json(serde_json::json!({ "success": true })))
}

pub fn apply_macos_vscode_window_setting(mode: &str, workspace_dir: Option<&Path>) {
    // 1. Zapisz w katalogu workspace (.vscode/settings.json) - natywna obsługa VS Code na każdym OS
    if let Some(ws) = workspace_dir {
        let ws_vscode = ws.join(".vscode");
        let _ = fs::create_dir_all(&ws_vscode);
        let s_file = ws_vscode.join("settings.json");
        let mut json_val: serde_json::Value = if s_file.exists() {
            fs::read_to_string(&s_file)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_else(|| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };
        if let Some(obj) = json_val.as_object_mut() {
            let dim_val = match mode {
                "maximized" => "maximized",
                "fullscreen" => "fullscreen",
                _ => "default",
            };
            obj.insert("window.newWindowDimensions".to_string(), serde_json::Value::String(dim_val.to_string()));
            if mode == "maximized" || mode == "fullscreen" {
                obj.insert("window.restoreWindows".to_string(), serde_json::Value::String("none".to_string()));
            }
            if let Ok(formatted) = serde_json::to_string_pretty(&json_val) {
                let _ = fs::write(&s_file, formatted);
            }
        }
    }

    // 2. Globalne ustawienia użytkownika w macOS Application Support
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let user_dirs = [
                home.join("Library/Application Support/Code/User"),
                home.join("Library/Application Support/VSCodium/User"),
                home.join("Library/Application Support/Code - Insiders/User"),
            ];
            for vscode_user_dir in &user_dirs {
                if vscode_user_dir.exists() {
                    let settings_file = vscode_user_dir.join("settings.json");
                    let mut json_val: serde_json::Value = if settings_file.exists() {
                        fs::read_to_string(&settings_file)
                            .ok()
                            .and_then(|c| serde_json::from_str(&c).ok())
                            .unwrap_or_else(|| serde_json::json!({}))
                    } else {
                        serde_json::json!({})
                    };

                    if let Some(obj) = json_val.as_object_mut() {
                        let dim_val = match mode {
                            "maximized" => "maximized",
                            "fullscreen" => "fullscreen",
                            _ => "default",
                        };
                        obj.insert("window.newWindowDimensions".to_string(), serde_json::Value::String(dim_val.to_string()));
                        if mode == "maximized" || mode == "fullscreen" {
                            obj.insert("window.restoreWindows".to_string(), serde_json::Value::String("none".to_string()));
                        }
                        if let Ok(formatted) = serde_json::to_string_pretty(&json_val) {
                            let _ = fs::write(&settings_file, formatted);
                        }
                    }
                }
            }
        }
    }
    let _ = mode;
}

fn apply_autostart_setting(enable: bool) {
    #[cfg(target_os = "linux")]
    {
        if enable {
            let _ = std::process::Command::new("systemctl").args(["--user", "enable", "algodeck.service"]).output();
        } else {
            let _ = std::process::Command::new("systemctl").args(["--user", "disable", "algodeck.service"]).output();
            if let Some(home) = dirs::home_dir() {
                let _ = std::fs::remove_file(home.join(".config/autostart/algodeck.desktop"));
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let startup_vbs = std::path::PathBuf::from(appdata).join("Microsoft/Windows/Start Menu/Programs/Startup/AlgoDeck.vbs");
            if enable {
                if let Ok(local) = std::env::var("LOCALAPPDATA") {
                    let src = std::path::PathBuf::from(local).join("AlgoDeck/AlgoDeck.vbs");
                    if src.exists() {
                        let _ = std::fs::copy(src, startup_vbs);
                    }
                }
            } else {
                let _ = std::fs::remove_file(startup_vbs);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let plist = home.join("Library/LaunchAgents/com.algodeck.daemon.plist");
            let uid = unsafe { libc::getuid() };
            let service_target = format!("gui/{}/com.algodeck.daemon", uid);
            if enable {
                let _ = std::process::Command::new("launchctl").args(["enable", &service_target]).output();
                let _ = std::process::Command::new("launchctl").args(["bootstrap", &format!("gui/{}", uid)]).arg(&plist).output();
                let _ = std::process::Command::new("launchctl").args(["load"]).arg(&plist).output();
            } else {
                let _ = std::process::Command::new("launchctl").args(["bootout", &service_target]).output();
                let _ = std::process::Command::new("launchctl").args(["unload"]).arg(&plist).output();
            }
        }
    }
}

fn which_bin_exists(bin: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for p in std::env::split_paths(&path) {
            let full = p.join(bin);
            if full.exists() {
                return true;
            }
        }
    }
    false
}

#[cfg(target_os = "linux")]
fn ensure_kde_focus_stealing_disabled() {
    static CHECKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if CHECKED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    if let Some(home) = dirs::home_dir() {
        let kwinrc = home.join(".config").join("kwinrc");
        if kwinrc.exists() {
            if let Ok(content) = fs::read_to_string(&kwinrc) {
                if !content.contains("FocusStealingPreventionSeverity=0") {
                    let _ = std::process::Command::new("kwriteconfig6")
                        .args(["--file", "kwinrc", "--group", "Windows", "--key", "FocusStealingPreventionSeverity", "0"])
                        .output();
                    let _ = std::process::Command::new("kwriteconfig5")
                        .args(["--file", "kwinrc", "--group", "Windows", "--key", "FocusStealingPreventionSeverity", "0"])
                        .output();
                    let _ = std::process::Command::new("dbus-send")
                        .args(["--session", "--dest=org.kde.KWin", "/KWin", "org.kde.KWin.reconfigure"])
                        .output();
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn configure_cmd_env(cmd: &mut std::process::Command) {
    if std::env::var("DISPLAY").is_err() {
        cmd.env("DISPLAY", ":0");
    }
    if std::env::var("XAUTHORITY").is_err() {
        if let Some(home) = dirs::home_dir() {
            let xauth = home.join(".Xauthority");
            if xauth.exists() {
                cmd.env("XAUTHORITY", xauth);
            }
        }
    }
    if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
        let uid = std::env::var("UID").unwrap_or_else(|_| (unsafe { libc::getuid() }).to_string());
        let bus = format!("unix:path=/run/user/{}/bus", uid);
        cmd.env("DBUS_SESSION_BUS_ADDRESS", bus);
    }
    if std::env::var("WAYLAND_DISPLAY").is_err() {
        if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
            if std::path::Path::new(&runtime).join("wayland-0").exists() {
                cmd.env("WAYLAND_DISPLAY", "wayland-0");
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn find_linux_tool_bin(bin: &str) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("PATH") {
        for p in std::env::split_paths(&path) {
            let cand = p.join(bin);
            if cand.exists() {
                return Some(cand);
            }
        }
    }
    if let Some(home) = dirs::home_dir() {
        let cand = home.join(".local/bin").join(bin);
        if cand.exists() {
            return Some(cand);
        }
    }
    let usr = PathBuf::from("/usr/bin").join(bin);
    if usr.exists() {
        return Some(usr);
    }
    None
}

#[cfg(target_os = "linux")]
pub fn activate_linux_window(target_classes: &[&str]) {
    ensure_kde_focus_stealing_disabled();

    // 1. KWin Scripting via DBus (Wayland & X11 for KDE Plasma)
    let mut js_conditions = Vec::new();
    for cls in target_classes {
        js_conditions.push(format!("cls.indexOf(\"{}\") !== -1", cls.to_lowercase()));
    }
    let condition_expr = js_conditions.join(" || ");

    let kwin_script = format!(
        r#"
var list = workspace.windowList ? workspace.windowList() : (workspace.clientList ? workspace.clientList() : []);
for (var i = 0; i < list.length; i++) {{
    var c = list[i];
    var cls = ((c.resourceClass || "") + " " + (c.resourceName || "") + " " + (c.caption || "")).toLowerCase();
    if ({condition}) {{
        c.minimized = false;
        c.demandsAttention = false;
        workspace.activeWindow = c;
        if (c.activateWindow) c.activateWindow();
        break;
    }}
}}
"#,
        condition = condition_expr
    );

    let tmp_script = std::env::temp_dir().join(format!("kwin_focus_{}.js", std::process::id()));
    if fs::write(&tmp_script, &kwin_script).is_ok() {
        let script_path = tmp_script.to_string_lossy().to_string();
        let mut dbus_cmd = std::process::Command::new("dbus-send");
        configure_cmd_env(&mut dbus_cmd);
        let load_res = dbus_cmd
            .args([
                "--session",
                "--print-reply=literal",
                "--dest=org.kde.KWin",
                "/Scripting",
                "org.kde.kwin.Scripting.loadScript",
                &format!("string:{}", script_path),
            ])
            .output();

        if let Ok(out) = load_res {
            let reply = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Some(script_id) = reply.split_whitespace().last() {
                if let Ok(num) = script_id.parse::<i32>() {
                    let obj_path = format!("/Scripting/Script{}", num);
                    let mut run_cmd = std::process::Command::new("dbus-send");
                    configure_cmd_env(&mut run_cmd);
                    let _ = run_cmd
                        .args([
                            "--session",
                            "--dest=org.kde.KWin",
                            &obj_path,
                            "org.kde.kwin.Script.run",
                        ])
                        .output();

                    let mut stop_cmd = std::process::Command::new("dbus-send");
                    configure_cmd_env(&mut stop_cmd);
                    let _ = stop_cmd
                        .args([
                            "--session",
                            "--dest=org.kde.KWin",
                            &obj_path,
                            "org.kde.kwin.Script.stop",
                        ])
                        .output();
                }
            }
        }
        let _ = fs::remove_file(tmp_script);
    }

    // 2. Cinnamon Desktop (Muffin) Window Manager via DBus Eval
    let cin_conditions: Vec<String> = target_classes
        .iter()
        .map(|c| format!("\"{}\"", c.to_lowercase()))
        .collect();
    let cin_targets_json = cin_conditions.join(", ");
    let cinnamon_eval_script = format!(
        r#"try {{
            let wins = global.display.get_tab_list(0, null);
            let targets = [{targets}];
            for (let w of wins) {{
                let cls = ((w.get_wm_class() || '') + ' ' + (w.get_title() || '')).toLowerCase();
                for (let t of targets) {{
                    if (cls.indexOf(t) !== -1) {{
                        w.unminimize();
                        w.activate(global.get_current_time());
                        w.raise();
                        break;
                    }}
                }}
            }}
        }} catch (_) {{}}"#,
        targets = cin_targets_json
    );

    let mut cin_cmd = std::process::Command::new("dbus-send");
    configure_cmd_env(&mut cin_cmd);
    let _ = cin_cmd
        .args([
            "--session",
            "--dest=org.Cinnamon",
            "--type=method_call",
            "/org/Cinnamon",
            "org.Cinnamon.Eval",
            &format!("string:{}", cinnamon_eval_script),
        ])
        .output();

    // 3. GNOME Shell Window Manager via DBus Eval
    let mut gnome_cmd = std::process::Command::new("dbus-send");
    configure_cmd_env(&mut gnome_cmd);
    let _ = gnome_cmd
        .args([
            "--session",
            "--dest=org.gnome.Shell",
            "--type=method_call",
            "/org/gnome/Shell",
            "org.gnome.Shell.Eval",
            &format!("string:{}", cinnamon_eval_script),
        ])
        .output();

    // 4. kdotool (KDE Plasma Wayland focus tool)
    if let Some(kdo) = find_linux_tool_bin("kdotool") {
        for cls in target_classes {
            let mut cmd = std::process::Command::new(&kdo);
            configure_cmd_env(&mut cmd);
            let _ = cmd.args(["search", "--class", cls, "windowactivate"]).output();
        }
    }

    // 5. wmctrl & xdotool: identify matching window IDs, remove demands_attention, and activate/raise
    let xd_opt = find_linux_tool_bin("xdotool");
    if let Some(wm) = find_linux_tool_bin("wmctrl") {
        let mut list_cmd = std::process::Command::new(&wm);
        configure_cmd_env(&mut list_cmd);
        if let Ok(out) = list_cmd.args(["-l", "-x"]).output() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let lower = line.to_lowercase();
                for cls in target_classes {
                    if lower.contains(&cls.to_lowercase()) {
                        if let Some(wid) = line.split_whitespace().next() {
                            let mut rem_cmd = std::process::Command::new(&wm);
                            configure_cmd_env(&mut rem_cmd);
                            let _ = rem_cmd.args(["-i", "-r", wid, "-b", "remove,demands_attention"]).output();

                            let mut act_cmd = std::process::Command::new(&wm);
                            configure_cmd_env(&mut act_cmd);
                            let _ = act_cmd.args(["-i", "-a", wid]).output();

                            if let Some(ref xd) = xd_opt {
                                let mut xd_cmd = std::process::Command::new(xd);
                                configure_cmd_env(&mut xd_cmd);
                                let _ = xd_cmd.args(["windowactivate", "--sync", wid, "windowraise", wid, "windowfocus", "--sync", wid]).output();
                            }
                        }
                    }
                }
            }
        }
        for cls in target_classes {
            let mut act = std::process::Command::new(&wm);
            configure_cmd_env(&mut act);
            let _ = act.args(["-x", "-a", cls]).output();
        }
    }

    // 6. xdotool fallback search & activate
    if let Some(xd) = xd_opt {
        for cls in target_classes {
            let mut cmd = std::process::Command::new(&xd);
            configure_cmd_env(&mut cmd);
            let _ = cmd.args(["search", "--onlyvisible", "--class", cls, "windowactivate", "%@", "windowraise", "%@", "windowfocus", "%@"]).output();
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn activate_linux_window(_target_classes: &[&str]) {}

pub fn open_browser_foreground(url: &str) {
    let url_str = url.to_string();

    #[cfg(target_os = "linux")]
    {
        let mut launched = false;
        let browsers = ["brave-browser", "brave", "google-chrome", "google-chrome-stable", "chromium", "firefox", "microsoft-edge", "opera", "vivaldi"];
        for b in browsers {
            if which_bin_exists(b) {
                let mut cmd = std::process::Command::new(b);
                configure_cmd_env(&mut cmd);
                if cmd.arg(&url_str).spawn().is_ok() {
                    launched = true;
                    break;
                }
            }
        }

        if !launched {
            if let Ok(output) = std::process::Command::new("xdg-settings").args(["get", "default-web-browser"]).output() {
                let def_desktop = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !def_desktop.is_empty() && def_desktop.ends_with(".desktop") {
                    let mut cmd = std::process::Command::new("gtk-launch");
                    configure_cmd_env(&mut cmd);
                    if cmd.arg(&def_desktop).arg(&url_str).spawn().is_ok() {
                        launched = true;
                    }
                }
            }
        }

        if !launched {
            let _ = open::that(&url_str);
        }

        activate_linux_window(&["brave-browser", "brave", "google-chrome", "chrome", "chromium", "firefox", "microsoft-edge", "vivaldi", "opera"]);

        std::thread::spawn(move || {
            for delay in [50, 150, 300, 600, 1000] {
                std::thread::sleep(std::time::Duration::from_millis(delay));
                activate_linux_window(&["brave-browser", "brave", "google-chrome", "chrome", "chromium", "firefox", "microsoft-edge", "vivaldi", "opera"]);
            }
        });
    }

    #[cfg(target_os = "macos")]
    {
        let _ = open::that(&url_str);
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));
            let script = r#"
                tell application "System Events"
                    set bList to {"Brave Browser", "Google Chrome", "Arc", "Chromium", "Safari", "Firefox", "Microsoft Edge"}
                    repeat with b in bList
                        if exists (process b) then
                            tell application b to activate
                            exit repeat
                        end if
                    end repeat
                end tell
            "#;
            let _ = std::process::Command::new("osascript").args(["-e", script]).output();
        });
    }

    #[cfg(target_os = "windows")]
    {
        let _ = open::that(&url_str);
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));
            let ps_cmd = r#"$w = New-Object -ComObject WScript.Shell; $p = Get-Process | Where-Object { $_.ProcessName -match 'chrome|brave|msedge|firefox|opera|vivaldi' } | Sort-Object -Descending -Property StartTime | Select-Object -First 1; if ($p) { $w.AppActivate($p.MainWindowTitle) }"#;
            let _ = std::process::Command::new("powershell").args(["-NoProfile", "-Command", ps_cmd]).spawn();
        });
    }
}

async fn open_gui() -> Json<serde_json::Value> {
    open_browser_foreground("http://127.0.0.1:8080/");
    Json(serde_json::json!({ "success": true }))
}

async fn get_problems(State(state): State<AppState>) -> Json<serde_json::Value> {
    let ws_dir = state.settings.read().unwrap().get_workspace_path();
    let list = WorkspaceManager::list_problems(&ws_dir);
    let active = state.streamdeck.get_active_problem_id();
    Json(serde_json::json!({
        "problems": list,
        "active": active
    }))
}

async fn get_problem(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<ProblemManifest>, StatusCode> {
    let ws_dir = state.settings.read().unwrap().get_workspace_path();
    match WorkspaceManager::get_manifest(&ws_dir, &id) {
        Some(m) => Ok(Json(m)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn delete_problem(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Json<serde_json::Value> {
    let ws_dir = state.settings.read().unwrap().get_workspace_path();
    let res = WorkspaceManager::delete_problem(&ws_dir, &id);
    Json(serde_json::json!({ "success": res.is_ok() }))
}

async fn add_tests(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let ws_dir = state.settings.read().unwrap().get_workspace_path();
    let mut zip_bytes = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        if let Ok(bytes) = field.bytes().await {
            zip_bytes = bytes.to_vec();
            break;
        }
    }

    if zip_bytes.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    match WorkspaceManager::add_tests_from_zip(&ws_dir, &id, &zip_bytes) {
        Ok(added) => Ok(Json(serde_json::json!({ "success": true, "tests_added": added }))),
        Err(e) => Ok(Json(serde_json::json!({ "success": false, "error": e }))),
    }
}

async fn auto_import_pdf(
    State(state): State<AppState>,
    Json(req): Json<AutoImportRequest>,
) -> Json<serde_json::Value> {
    let mut path = PathBuf::from(&req.file_path);
    if !path.exists() {
        if let Some(fname) = Path::new(&req.file_path).file_name() {
            if let Some(dl) = dirs::download_dir() {
                let candidate = dl.join(fname);
                if candidate.exists() {
                    path = candidate;
                }
            }
        }
    }

    if !path.exists() {
        return Json(serde_json::json!({ "success": false, "detail": "Plik zadania nie istnieje na dysku" }));
    }

    let initial = crate::config::InitialDownloads::load_or_init();
    if initial.is_ignored(&path) {
        println!("[AlgoDeck] Pominięto auto_import_pdf {:?} - plik istniał przed instalacją AlgoDeck", path);
        return Json(serde_json::json!({
            "success": false,
            "ignored": true,
            "detail": "Plik zignorowany jako istniejący przed instalacją AlgoDeck"
        }));
    }

    let is_pdf = path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("pdf")).unwrap_or(false);
    let is_zip = path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("zip")).unwrap_or(false);

    let mut analysis = if is_pdf {
        let text = match PdfParser::extract_text_from_file(&path) {
            Ok(t) => t,
            Err(e) => return Json(serde_json::json!({ "success": false, "detail": e })),
        };
        PdfParser::parse_heuristics(&text)
    } else {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("zad").to_lowercase();
        ProblemAnalysis {
            problem_id: stem.clone(),
            title: stem,
            time_limit_sec: 1.0,
            memory_limit_mb: 128,
            tests: Vec::new(),
            used_engine: "Import z archiwum ZIP".to_string(),
        }
    };

    if let Some(pid) = req.problem_id {
        if !pid.trim().is_empty() {
            analysis.problem_id = pid.trim().to_lowercase();
        }
    }
    if let Some(t) = req.title {
        if !t.trim().is_empty() {
            analysis.title = t.trim().to_string();
        }
    }
    if let Some(tl) = req.time_limit {
        analysis.time_limit_sec = tl;
    }
    if let Some(ml) = req.memory_limit {
        analysis.memory_limit_mb = ml;
    }

    if req.auto_create {
        let ws_dir = state.settings.read().unwrap().get_workspace_path();
        let pdf_arg = if is_pdf { Some(path.as_path()) } else { None };
        match WorkspaceManager::create_problem(&ws_dir, &analysis, pdf_arg) {
            Ok(_) => {
                if is_zip {
                    if let Ok(bytes) = fs::read(&path) {
                        let _ = WorkspaceManager::add_tests_from_zip(&ws_dir, &analysis.problem_id, &bytes);
                    }
                }
                let problems = WorkspaceManager::list_problems(&ws_dir);
                let total = problems.len().max(1);
                let cur_idx = problems.iter().position(|p| p.problem_id.eq_ignore_ascii_case(&analysis.problem_id)).map(|i| i + 1).unwrap_or(total);
                state.streamdeck.set_active_problem_with_meta(&analysis.problem_id, cur_idx, total);
                let _ = state.ws_sender.send(format!("active:{}", analysis.problem_id));
                let (split_pdf, pdf_pos, split_ratio, vscode_mode, macos_mode) = {
                    let s = state.settings.read().unwrap();
                    (s.open_pdf_in_vscode && is_pdf, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone(), s.macos_window_mode.clone())
                };
                open_in_vscode_internal(&analysis.problem_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode, &macos_mode);
                Json(serde_json::json!({
                    "success": true,
                    "problem_id": analysis.problem_id,
                    "title": analysis.title,
                    "time_limit_sec": analysis.time_limit_sec,
                    "memory_limit_mb": analysis.memory_limit_mb,
                    "tests_count": analysis.tests.len(),
                }))
            }
            Err(e) => Json(serde_json::json!({ "success": false, "detail": e })),
        }
    } else {
        Json(serde_json::json!({
            "success": true,
            "problem_id": analysis.problem_id,
            "title": analysis.title,
            "time_limit_sec": analysis.time_limit_sec,
            "memory_limit_mb": analysis.memory_limit_mb,
            "tests_count": analysis.tests.len(),
        }))
    }
}

async fn create_manual(
    State(state): State<AppState>,
    Json(req): Json<ManualTaskRequest>,
) -> Json<serde_json::Value> {
    let pid = req.problem_id.trim().to_lowercase();
    let mut tests = Vec::new();

    // 1. Zapisz testy przekazane w tablicy tests (z popup.js lub app.js)
    for (i, t) in req.tests.into_iter().enumerate() {
        if !t.input.trim().is_empty() || !t.expected_output.trim().is_empty() {
            let tid = t.id.unwrap_or_else(|| format!("test_{}", i + 1));
            let tname = t.name.unwrap_or_else(|| format!("Przykład {}", i + 1));
            tests.push(TestCase {
                id: tid,
                name: tname,
                input: t.input,
                expected_output: t.expected_output,
                is_edge_case: false,
                tag: "[RĘCZNY]".to_string(),
            });
        }
    }

    // 2. Jeśli podano także w polach sample_cin / sample_cout i tablica tests jest pusta:
    if (!req.sample_cin.trim().is_empty() || !req.sample_cout.trim().is_empty()) && tests.is_empty() {
        tests.push(TestCase {
            id: "test_1".to_string(),
            name: "Przykład 1".to_string(),
            input: req.sample_cin.trim().to_string(),
            expected_output: req.sample_cout.trim().to_string(),
            is_edge_case: false,
            tag: "[RĘCZNY]".to_string(),
        });
    }

    let analysis = ProblemAnalysis {
        problem_id: pid.clone(),
        title: req.title.trim().to_string(),
        time_limit_sec: req.time_limit,
        memory_limit_mb: req.memory_limit,
        tests,
        used_engine: "Ręczny kreator zadania".to_string(),
    };

    let ws_dir = state.settings.read().unwrap().get_workspace_path();
    match WorkspaceManager::create_problem(&ws_dir, &analysis, None) {
        Ok(_) => {
            let problems = WorkspaceManager::list_problems(&ws_dir);
            let total = problems.len().max(1);
            let cur_idx = problems.iter().position(|p| p.problem_id.eq_ignore_ascii_case(&pid)).map(|i| i + 1).unwrap_or(total);
            state.streamdeck.set_active_problem_with_meta(&pid, cur_idx, total);
            let _ = state.ws_sender.send(format!("active:{}", pid));
            let (split_pdf, pdf_pos, split_ratio, vscode_mode, macos_mode) = {
                let s = state.settings.read().unwrap();
                (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone(), s.macos_window_mode.clone())
            };
            open_in_vscode_internal(&pid, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode, &macos_mode);
            Json(serde_json::json!({ "success": true, "problem_id": pid }))
        }
        Err(e) => Json(serde_json::json!({ "success": false, "detail": e })),
    }
}

async fn get_staged_state() -> Json<serde_json::Value> {
    let stage_dir = AppSettings::staging_dir();
    let mut pdf_path: Option<String> = None;
    let mut pdf_filename: Option<String> = None;
    let mut zip_path: Option<String> = None;
    let mut zip_filename: Option<String> = None;
    let mut problem_id: Option<String> = None;
    let mut title: Option<String> = None;
    let mut time_limit_sec: Option<f64> = None;
    let mut memory_limit_mb: Option<u64> = None;
    let mut tests_count: usize = 0;

    if stage_dir.exists() {
        if let Ok(entries) = fs::read_dir(&stage_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                    if ext.eq_ignore_ascii_case("pdf") {
                        pdf_path = Some(p.to_string_lossy().to_string());
                        pdf_filename = p.file_name().map(|f| f.to_string_lossy().to_string());
                        if let Ok(text) = PdfParser::extract_text_from_file(&p) {
                            let analysis = PdfParser::parse_heuristics(&text);
                            problem_id = Some(analysis.problem_id);
                            title = Some(analysis.title);
                            time_limit_sec = Some(analysis.time_limit_sec);
                            memory_limit_mb = Some(analysis.memory_limit_mb);
                            tests_count = analysis.tests.len();
                        }
                    } else if ext.eq_ignore_ascii_case("zip") {
                        zip_path = Some(p.to_string_lossy().to_string());
                        zip_filename = p.file_name().map(|f| f.to_string_lossy().to_string());
                        if let Ok(bytes) = fs::read(&p) {
                            let reader = std::io::Cursor::new(bytes);
                            if let Ok(mut archive) = zip::ZipArchive::new(reader) {
                                for i in 0..archive.len() {
                                    if let Ok(file) = archive.by_index(i) {
                                        if file.name().to_lowercase().ends_with(".in") {
                                            tests_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Json(serde_json::json!({
        "success": true,
        "has_pdf": pdf_path.is_some(),
        "has_staged_pdf": pdf_path.is_some(),
        "pdf_path": pdf_path,
        "pdf_filename": pdf_filename,
        "has_zip": zip_path.is_some(),
        "has_staged_zip": zip_path.is_some(),
        "zip_path": zip_path,
        "zip_filename": zip_filename,
        "problem_id": problem_id,
        "title": title,
        "time_limit_sec": time_limit_sec.unwrap_or(1.0),
        "memory_limit_mb": memory_limit_mb.unwrap_or(128),
        "tests_count": tests_count,
    }))
}

async fn stage_file(
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let stage_dir = AppSettings::staging_dir();
    let _ = fs::create_dir_all(&stage_dir);

    let mut saved_filename = String::new();
    let mut file_type = String::new();
    let mut target_path = PathBuf::new();
    let mut file_bytes = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file_type" {
            if let Ok(text) = field.text().await {
                file_type = text.trim().to_lowercase();
            }
        } else if name == "file" || field.file_name().is_some() {
            let fname = field.file_name().unwrap_or("staged_file").to_string();
            let clean_fname = Path::new(&fname)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "staged_file".to_string());
            saved_filename = clean_fname.clone();
            target_path = stage_dir.join(&clean_fname);
            if let Ok(bytes) = field.bytes().await {
                file_bytes = bytes.to_vec();
                let _ = fs::write(&target_path, &file_bytes);
            }
        }
    }

    if saved_filename.is_empty() || file_bytes.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "detail": "Nie przesłano poprawnego pliku" })),
        ));
    }

    let is_pdf = file_type == "pdf" || saved_filename.to_lowercase().ends_with(".pdf");
    let is_zip = file_type == "zip" || saved_filename.to_lowercase().ends_with(".zip");

    let mut problem_id = None;
    let mut title = None;
    let mut time_limit_sec = 1.0;
    let mut memory_limit_mb = 128;
    let mut tests_count = 0;

    if is_pdf {
        if let Ok(text) = PdfParser::extract_text_from_file(&target_path) {
            let analysis = PdfParser::parse_heuristics(&text);
            problem_id = Some(analysis.problem_id);
            title = Some(analysis.title);
            time_limit_sec = analysis.time_limit_sec;
            memory_limit_mb = analysis.memory_limit_mb;
            tests_count = analysis.tests.len();
        }
    } else if is_zip {
        let reader = std::io::Cursor::new(&file_bytes);
        if let Ok(mut archive) = zip::ZipArchive::new(reader) {
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    if file.name().to_lowercase().ends_with(".in") {
                        tests_count += 1;
                    }
                }
            }
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "filename": saved_filename,
        "file_type": if is_pdf { "pdf" } else if is_zip { "zip" } else { "other" },
        "path": target_path.to_string_lossy(),
        "has_pdf": is_pdf,
        "has_staged_pdf": is_pdf,
        "pdf_filename": if is_pdf { Some(&saved_filename) } else { None },
        "has_zip": is_zip,
        "has_staged_zip": is_zip,
        "zip_filename": if is_zip { Some(&saved_filename) } else { None },
        "problem_id": problem_id,
        "title": title,
        "time_limit_sec": time_limit_sec,
        "memory_limit_mb": memory_limit_mb,
        "tests_count": tests_count,
    })))
}

#[derive(Deserialize)]
struct StagePathRequest {
    file_path: String,
}

fn url_decode_path(s: &str) -> String {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
                out.push(val);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

pub fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' || b == b'/' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

async fn stage_by_path(
    Json(req): Json<StagePathRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let raw = req.file_path.trim().trim_start_matches("file://");
    let decoded = url_decode_path(raw);
    let expand_tilde = |p: &str| -> PathBuf {
        if p.starts_with("~/") || p == "~" {
            dirs::home_dir().map(|h| h.join(p.trim_start_matches("~/"))).unwrap_or_else(|| PathBuf::from(p))
        } else {
            PathBuf::from(p)
        }
    };

    let mut path = expand_tilde(raw);
    if !path.exists() {
        let dec_path = expand_tilde(&decoded);
        if dec_path.exists() {
            path = dec_path;
        } else {
            let fname_opt = path.file_name().or_else(|| Path::new(&decoded).file_name()).map(|f| f.to_os_string());
            if let Some(fname) = fname_opt {
                for dl in crate::config::get_all_download_dirs() {
                    let candidate = dl.join(&fname);
                    if candidate.exists() {
                        path = candidate;
                        break;
                    }
                }
            }
        }
    }

    if path.exists() {
        if let Some(parent) = path.parent() {
            crate::config::register_known_download_dir(parent);
        }
    }

    if !path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "success": false, "detail": "Plik nie istnieje na dysku" })),
        ));
    }

    let initial = crate::config::InitialDownloads::load_or_init();
    if initial.is_ignored(&path) {
        println!("[AlgoDeck] Pominięto stage_by_path {:?} - plik istniał przed instalacją AlgoDeck", path);
        return Ok(Json(serde_json::json!({
            "success": false,
            "ignored": true,
            "detail": "Plik zignorowany jako istniejący przed instalacją AlgoDeck"
        })));
    }

    let mut is_pdf = path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("pdf")).unwrap_or(false);
    let is_zip = path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("zip")).unwrap_or(false);

    if !is_pdf && !is_zip && path.exists() {
        if let Ok(file_bytes) = fs::read(&path) {
            if file_bytes.starts_with(b"%PDF") {
                is_pdf = true;
            }
        }
    }

    if is_pdf {
        let mut reg = crate::config::DownloadedPdfRegistry::load_or_init();
        if !reg.check_and_register_if_new(&path) {
            println!("[AlgoDeck] Pominięto stage_by_path {:?} - plik już w rejestrze pobranych", path);
            return Ok(Json(serde_json::json!({
                "success": false,
                "ignored": true,
                "detail": "Plik zignorowany jako znany w rejestrze pobranych"
            })));
        }
    }

    if !is_pdf && !is_zip {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "detail": "Wymagany plik PDF lub ZIP" })),
        ));
    }

    let stage_dir = AppSettings::staging_dir();
    let _ = fs::create_dir_all(&stage_dir);

    // Zawsze zachowaj tylko OSTATNI plik danego typu (zgodnie z życzeniem użytkownika)
    if let Ok(entries) = fs::read_dir(&stage_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                if is_pdf && ext.eq_ignore_ascii_case("pdf") {
                    let _ = fs::remove_file(p);
                } else if is_zip && ext.eq_ignore_ascii_case("zip") {
                    let _ = fs::remove_file(p);
                }
            }
        }
    }

    let mut filename = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "downloaded_file".to_string());
    if is_pdf && !filename.to_lowercase().ends_with(".pdf") {
        filename = format!("{}.pdf", filename);
    }
    let target_path = stage_dir.join(&filename);

    if let Err(e) = fs::copy(&path, &target_path) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "success": false, "detail": format!("Błąd kopiowania pliku do poczekalni: {}", e) })),
        ));
    }

    let mut problem_id = None;
    let mut title = None;
    let mut time_limit_sec = 1.0;
    let mut memory_limit_mb = 128;
    let mut tests_count = 0;

    if is_pdf {
        if let Ok(text) = PdfParser::extract_text_from_file(&target_path) {
            let analysis = PdfParser::parse_heuristics(&text);
            problem_id = Some(analysis.problem_id);
            title = Some(analysis.title);
            time_limit_sec = analysis.time_limit_sec;
            memory_limit_mb = analysis.memory_limit_mb;
            tests_count = analysis.tests.len();
        }
    } else if is_zip {
        if let Ok(file_bytes) = fs::read(&target_path) {
            let reader = std::io::Cursor::new(&file_bytes);
            if let Ok(mut archive) = zip::ZipArchive::new(reader) {
                for i in 0..archive.len() {
                    if let Ok(file) = archive.by_index(i) {
                        if file.name().to_lowercase().ends_with(".in") {
                            tests_count += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "filename": filename,
        "file_type": if is_pdf { "pdf" } else { "zip" },
        "path": target_path.to_string_lossy(),
        "has_pdf": is_pdf,
        "has_staged_pdf": is_pdf,
        "pdf_filename": if is_pdf { Some(&filename) } else { None },
        "has_zip": is_zip,
        "has_staged_zip": is_zip,
        "zip_filename": if is_zip { Some(&filename) } else { None },
        "problem_id": problem_id,
        "title": title,
        "time_limit_sec": time_limit_sec,
        "memory_limit_mb": memory_limit_mb,
        "tests_count": tests_count,
    })))
}

async fn get_registered_downloads() -> Json<serde_json::Value> {
    let reg = crate::config::DownloadedPdfRegistry::load_or_init();
    Json(serde_json::to_value(&reg).unwrap_or_else(|_| serde_json::json!({ "version": 1, "entries": {} })))
}

async fn clear_download_registry() -> Json<serde_json::Value> {
    let mut reg = crate::config::DownloadedPdfRegistry::load_or_init();
    let _ = reg.clear();

    let mut init = crate::config::InitialDownloads::load_or_init();
    let _ = init.clear();

    let stage_dir = AppSettings::staging_dir();
    if stage_dir.exists() {
        if let Ok(entries) = fs::read_dir(&stage_dir) {
            for e in entries.flatten() {
                let _ = fs::remove_file(e.path());
            }
        }
    }

    Json(serde_json::json!({
        "success": true,
        "message": "Pomyślnie wyczyszczono rejestr pobranych zadań PDF i pamięć ignorowania."
    }))
}

#[derive(Debug, Deserialize)]
pub struct RegisterDirsRequest {
    pub directories: Vec<String>,
}

async fn register_download_dirs(
    Json(req): Json<RegisterDirsRequest>,
) -> Json<serde_json::Value> {
    for d in req.directories {
        let p = PathBuf::from(d);
        if p.exists() && p.is_dir() {
            crate::config::register_known_download_dir(&p);
        }
    }

    let all = crate::config::get_all_download_dirs();
    let paths: Vec<String> = all.into_iter().map(|p| p.to_string_lossy().to_string()).collect();
    Json(serde_json::json!({ "success": true, "download_dirs": paths }))
}

async fn register_download(
    State(state): State<AppState>,
    Json(req): Json<StagePathRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let raw = req.file_path.trim().trim_start_matches("file://");
    let decoded = url_decode_path(raw);
    let expand_tilde = |p: &str| -> PathBuf {
        if p.starts_with("~/") || p == "~" {
            dirs::home_dir().map(|h| h.join(p.trim_start_matches("~/"))).unwrap_or_else(|| PathBuf::from(p))
        } else {
            PathBuf::from(p)
        }
    };

    let mut path = expand_tilde(raw);
    if !path.exists() {
        let dec_path = expand_tilde(&decoded);
        if dec_path.exists() {
            path = dec_path;
        } else {
            let fname_opt = path.file_name().or_else(|| Path::new(&decoded).file_name()).map(|f| f.to_os_string());
            if let Some(fname) = fname_opt {
                for dl in crate::config::get_all_download_dirs() {
                    let candidate = dl.join(&fname);
                    if candidate.exists() {
                        path = candidate;
                        break;
                    }
                }
            }
        }
    }

    if path.exists() {
        if let Some(parent) = path.parent() {
            crate::config::register_known_download_dir(parent);
        }
    }

    if !path.exists() {
        return Ok(Json(serde_json::json!({
            "success": false,
            "is_new": false,
            "detail": "Plik nie istnieje na dysku"
        })));
    }

    let mut is_pdf = path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("pdf")).unwrap_or(false);
    if !is_pdf && path.exists() {
        if let Ok(file_bytes) = fs::read(&path) {
            if file_bytes.starts_with(b"%PDF") {
                is_pdf = true;
            }
        }
    }

    if !is_pdf {
        return Ok(Json(serde_json::json!({
            "success": false,
            "is_new": false,
            "detail": "Rejestr pobranych dotyczy wyłącznie plików PDF"
        })));
    }

    let mut reg = crate::config::DownloadedPdfRegistry::load_or_init();
    let is_new = reg.check_and_register_if_new(&path);
    if !is_new {
        return Ok(Json(serde_json::json!({
            "success": true,
            "is_new": false,
            "staged": false,
            "detail": "Plik już zarejestrowany w pamięci trwałej"
        })));
    }

    let stage_dir = AppSettings::staging_dir();
    let _ = fs::create_dir_all(&stage_dir);

    if let Ok(entries) = fs::read_dir(&stage_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                if ext.eq_ignore_ascii_case("pdf") {
                    let _ = fs::remove_file(p);
                }
            }
        }
    }

    let mut filename = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "downloaded_file.pdf".to_string());
    if !filename.to_lowercase().ends_with(".pdf") {
        filename = format!("{}.pdf", filename);
    }
    let target_path = stage_dir.join(&filename);
    let _ = fs::copy(&path, &target_path);

    let mut problem_id = None;
    let mut title = None;
    let mut time_limit_sec = 1.0;
    let mut memory_limit_mb = 128;
    let mut tests_count = 0;

    if let Ok(text) = PdfParser::extract_text_from_file(&target_path) {
        let analysis = PdfParser::parse_heuristics(&text);
        problem_id = Some(analysis.problem_id);
        title = Some(analysis.title);
        time_limit_sec = analysis.time_limit_sec;
        memory_limit_mb = analysis.memory_limit_mb;
        tests_count = analysis.tests.len();
    }

    let (notifications_enabled, notify_new_task) = {
        let s = state.settings.read().unwrap();
        (s.notifications, s.notify_new_task_download)
    };

    let should_notify = notifications_enabled && notify_new_task;


    Ok(Json(serde_json::json!({
        "success": true,
        "is_new": true,
        "staged": true,
        "filename": filename,
        "path": target_path.to_string_lossy(),
        "problem_id": problem_id,
        "title": title,
        "time_limit_sec": time_limit_sec,
        "memory_limit_mb": memory_limit_mb,
        "tests_count": tests_count,
        "should_notify": should_notify
    })))
}

async fn clear_staged(
    body: Option<Json<ClearStagedRequest>>,
) -> Json<serde_json::Value> {
    let stage_dir = AppSettings::staging_dir();
    let filter_type = body.and_then(|b| b.file_type.clone());

    if stage_dir.exists() {
        if let Ok(entries) = fs::read_dir(&stage_dir) {
            for e in entries.flatten() {
                let p = e.path();
                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                let should_remove = match filter_type.as_deref() {
                    Some("pdf") => ext == "pdf",
                    Some("zip") => ext == "zip",
                    _ => true,
                };
                if should_remove {
                    let _ = fs::remove_file(p);
                }
            }
        }
    }

    get_staged_state().await
}

async fn start_staged(
    State(state): State<AppState>,
    body: Option<Json<serde_json::Value>>,
) -> Json<serde_json::Value> {
    let stage_dir = AppSettings::staging_dir();
    let mut pdf_file: Option<PathBuf> = None;
    let mut zip_file: Option<PathBuf> = None;

    if stage_dir.exists() {
        if let Ok(entries) = fs::read_dir(&stage_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                    if ext.eq_ignore_ascii_case("pdf") {
                        pdf_file = Some(p);
                    } else if ext.eq_ignore_ascii_case("zip") {
                        zip_file = Some(p);
                    }
                }
            }
        }
    }

    if pdf_file.is_none() && zip_file.is_none() {
        return Json(serde_json::json!({ "success": false, "error": "Brak plików w stagingu" }));
    }

    let mut custom_pid: Option<String> = None;
    let mut custom_title: Option<String> = None;
    if let Some(Json(ref b)) = body {
        if let Some(pid) = b.get("problem_id").and_then(|v| v.as_str()) {
            if !pid.trim().is_empty() {
                custom_pid = Some(pid.trim().to_lowercase());
            }
        }
        if let Some(t) = b.get("title").and_then(|v| v.as_str()) {
            if !t.trim().is_empty() {
                custom_title = Some(t.trim().to_string());
            }
        }
    }

    let ws_dir = state.settings.read().unwrap().get_workspace_path();

    if let Some(ref pdf) = pdf_file {
        if let Ok(text) = PdfParser::extract_text_from_file(pdf) {
            let mut analysis = PdfParser::parse_heuristics(&text);
            if let Some(ref pid) = custom_pid {
                analysis.problem_id = pid.clone();
            }
            if let Some(ref t) = custom_title {
                analysis.title = t.clone();
            }

            let _p_dir = match WorkspaceManager::create_problem(&ws_dir, &analysis, Some(pdf)) {
                Ok(p) => p,
                Err(e) => return Json(serde_json::json!({ "success": false, "error": e })),
            };

            if let Some(ref zip) = zip_file {
                if let Ok(bytes) = fs::read(zip) {
                    let _ = WorkspaceManager::add_tests_from_zip(&ws_dir, &analysis.problem_id, &bytes);
                }
            }

            let problems = WorkspaceManager::list_problems(&ws_dir);
            let total = problems.len().max(1);
            let cur_idx = problems.iter().position(|p| p.problem_id.eq_ignore_ascii_case(&analysis.problem_id)).map(|i| i + 1).unwrap_or(total);
            state.streamdeck.set_active_problem_with_meta(&analysis.problem_id, cur_idx, total);
            let _ = state.ws_sender.send(format!("active:{}", analysis.problem_id));
            let (split_pdf, pdf_pos, split_ratio, vscode_mode, macos_mode) = {
                let s = state.settings.read().unwrap();
                (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone(), s.macos_window_mode.clone())
            };
            open_in_vscode_internal(&analysis.problem_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode, &macos_mode);
            let _ = clear_staged(None).await;

            return Json(serde_json::json!({
                "success": true,
                "problem_id": analysis.problem_id,
                "title": analysis.title,
            }));
        }
    } else if let Some(ref zip) = zip_file {
        let fallback_pid = custom_pid.unwrap_or_else(|| {
            zip.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("zad")
                .to_lowercase()
        });
        let fallback_title = custom_title.unwrap_or_else(|| fallback_pid.clone());

        let analysis = ProblemAnalysis {
            problem_id: fallback_pid.clone(),
            title: fallback_title.clone(),
            time_limit_sec: 1.0,
            memory_limit_mb: 128,
            tests: Vec::new(),
            used_engine: "Import z archiwum ZIP".to_string(),
        };

        let _p_dir = match WorkspaceManager::create_problem(&ws_dir, &analysis, None) {
            Ok(p) => p,
            Err(e) => return Json(serde_json::json!({ "success": false, "error": e })),
        };

        if let Ok(bytes) = fs::read(zip) {
            let _ = WorkspaceManager::add_tests_from_zip(&ws_dir, &analysis.problem_id, &bytes);
        }

        let problems = WorkspaceManager::list_problems(&ws_dir);
        let total = problems.len().max(1);
        let cur_idx = problems.iter().position(|p| p.problem_id.eq_ignore_ascii_case(&analysis.problem_id)).map(|i| i + 1).unwrap_or(total);
        state.streamdeck.set_active_problem_with_meta(&analysis.problem_id, cur_idx, total);
        let _ = state.ws_sender.send(format!("active:{}", analysis.problem_id));
        let (split_pdf, pdf_pos, split_ratio, vscode_mode, macos_mode) = {
            let s = state.settings.read().unwrap();
            (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone(), s.macos_window_mode.clone())
        };
        open_in_vscode_internal(&analysis.problem_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode, &macos_mode);
        let _ = clear_staged(None).await;

        return Json(serde_json::json!({
            "success": true,
            "problem_id": analysis.problem_id,
            "title": analysis.title,
        }));
    }

    Json(serde_json::json!({ "success": false, "error": "Nie udało się sparsować pliku" }))
}

async fn set_active(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Json<serde_json::Value> {
    let ws_dir = state.settings.read().unwrap().get_workspace_path();
    let problems = WorkspaceManager::list_problems(&ws_dir);
    let total = problems.len().max(1);
    let cur_idx = problems.iter().position(|p| p.problem_id.eq_ignore_ascii_case(&id)).map(|i| i + 1).unwrap_or(total);
    state.streamdeck.set_active_problem_with_meta(&id, cur_idx, total);
    let _ = state.ws_sender.send(format!("active:{}", id));
    Json(serde_json::json!({ "success": true, "problem_id": id, "current_idx": cur_idx, "total": total }))
}

async fn open_vscode(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Json<serde_json::Value> {
    let ws_dir = state.settings.read().unwrap().get_workspace_path();
    let problems = WorkspaceManager::list_problems(&ws_dir);
    let total = problems.len().max(1);
    let cur_idx = problems.iter().position(|p| p.problem_id.eq_ignore_ascii_case(&id)).map(|i| i + 1).unwrap_or(total);
    state.streamdeck.set_active_problem_with_meta(&id, cur_idx, total);
    let _ = state.ws_sender.send(format!("active:{}", id));

    let (split_pdf, pdf_pos, split_ratio, vscode_mode, macos_mode) = {
        let s = state.settings.read().unwrap();
        (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone(), s.macos_window_mode.clone())
    };
    open_in_vscode_internal(&id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode, &macos_mode);
    Json(serde_json::json!({ "success": true }))
}

async fn stage_refresh() -> Json<serde_json::Value> {
    get_staged_state().await
}

fn register_in_vscode_extensions_json(ext_dir: &std::path::Path, rel_location: &str) {
    if let Some(parent) = ext_dir.parent() {
        let extensions_json_path = parent.join("extensions.json");
        let obsolete_json_path = parent.join(".obsolete");

        // 1. Usuń wpisy algodeck z .obsolete
        if obsolete_json_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&obsolete_json_path) {
                if content.contains("algodeck") {
                    if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(obj) = val.as_object_mut() {
                            obj.retain(|k, _| !k.contains("algodeck"));
                        }
                        if let Ok(new_content) = serde_json::to_string(&val) {
                            let _ = std::fs::write(&obsolete_json_path, new_content);
                        }
                    }
                }
            }
        }

        // 2. Zaktualizuj lub dodaj wpis w extensions.json
        let dest_str = ext_dir.to_string_lossy().to_string();
        let mut extensions_list: Vec<serde_json::Value> = if extensions_json_path.exists() {
            std::fs::read_to_string(&extensions_json_path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Usuń stare wpisy algodeck
        extensions_list.retain(|item| {
            if let Some(id) = item.pointer("/identifier/id").and_then(|v| v.as_str()) {
                if id == "algodeck.algodeck-vscode-bridge" {
                    return false;
                }
            }
            if let Some(rel) = item.get("relativeLocation").and_then(|v| v.as_str()) {
                if rel.contains("algodeck") {
                    return false;
                }
            }
            true
        });

        // Dodaj aktualny wpis dla wersji 2.1.0
        let new_entry = serde_json::json!({
            "identifier": {
                "id": "algodeck.algodeck-vscode-bridge"
            },
            "version": "2.1.0",
            "location": {
                "$mid": 1,
                "fsPath": dest_str,
                "path": dest_str,
                "scheme": "file"
            },
            "relativeLocation": rel_location,
            "metadata": {
                "installedTimestamp": 1729000000000u64,
                "source": "gallery"
            }
        });

        extensions_list.push(new_entry);

        if let Ok(serialized) = serde_json::to_string_pretty(&extensions_list) {
            let _ = std::fs::write(&extensions_json_path, serialized);
        }
    }
}

pub fn ensure_vscode_extension_installed() {
    if let Some(home) = dirs::home_dir() {
        let ext_roots = [
            home.join(".vscode/extensions"),
            home.join(".vscode-oss/extensions"),
            home.join(".var/app/com.visualstudio.code/data/vscode/extensions"),
            home.join(".var/app/com.vscodium.codium/data/vscode/extensions"),
            home.join("snap/code/common/.vscode/extensions"),
        ];

        let pkg = include_str!("../vscode_extension/package.json");
        let ext = include_str!("../vscode_extension/extension.js");

        for root in &ext_roots {
            let should_process = root == &ext_roots[0] || root.parent().map(|p| p.exists()).unwrap_or(false);
            if !should_process {
                continue;
            }

            let versioned_dir = root.join("algodeck.algodeck-vscode-bridge-2.1.0");
            let unversioned_dir = root.join("algodeck-vscode-bridge");

            for dest in [&versioned_dir, &unversioned_dir] {
                let ext_file = dest.join("extension.js");
                let pkg_file = dest.join("package.json");
                let should_update = match std::fs::read_to_string(&ext_file) {
                    Ok(existing) => existing != ext,
                    Err(_) => true,
                };
                if should_update {
                    let _ = std::fs::create_dir_all(dest);
                    let _ = std::fs::write(&pkg_file, pkg);
                    let _ = std::fs::write(&ext_file, ext);
                    println!("✓ Zainstalowano/zaktualizowano integrację VS Code Bridge w {}", dest.display());
                }
            }

            // Usuń przestarzały katalog 2.0.0
            let old_2_0 = root.join("algodeck.algodeck-vscode-bridge-2.0.0");
            if old_2_0.exists() {
                let _ = std::fs::remove_dir_all(&old_2_0);
            }

            // Zarejestruj w extensions.json i wyczyść .obsolete
            register_in_vscode_extensions_json(&versioned_dir, "algodeck.algodeck-vscode-bridge-2.1.0");
        }
    }
}

pub fn open_in_vscode_internal(
    problem_id: &str,
    workspace_dir: &Path,
    split_pdf: bool,
    pdf_position: &str,
    _split_ratio: u8,
    vscode_mode: &str,
    macos_window_mode: &str,
) {
    let p_dir = WorkspaceManager::get_problem_dir(workspace_dir, problem_id);
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

    // Upewnij się, że zadanie posiada skrypty i .vscode/tasks.json (tylko jeśli jeszcze nie istnieją)
    if !p_dir.join(".algo").join("run.sh").exists() {
        WorkspaceManager::ensure_problem_scripts(workspace_dir, problem_id, problem_id, 1.0);
    }

    // Ustaw natywne parametry okna VS Code w .vscode/settings.json
    apply_macos_vscode_window_setting(macos_window_mode, Some(workspace_dir));

    // 1. Spróbuj otworzyć plik natychmiast przez mostek IPC bez uruchamiania procesów CLI
    let sol_str = sol_file.to_string_lossy().to_string();
    let p_dir_str = p_dir.to_string_lossy().to_string();
    if send_bridge_open(problem_id, &p_dir_str, &sol_str, vscode_mode) {
        return;
    }

    static LAST_VSCODE_LAUNCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let last = LAST_VSCODE_LAUNCH.swap(now, std::sync::atomic::Ordering::Relaxed);
    if now.saturating_sub(last) < 600 {
        return;
    }

    let pdf_on_left = pdf_position == "left";
    let is_separate = vscode_mode == "separate_windows";
    let target_dir = if is_separate { &p_dir } else { workspace_dir };

    #[cfg(target_os = "windows")]
    {
        let mut args = Vec::new();
        args.push("--reuse-window".to_string());
        args.push(target_dir.to_string_lossy().to_string());
        if split_pdf && pdf_on_left {
            if let Some(ref p) = actual_pdf {
                args.push(p.to_string_lossy().to_string());
            }
        }
        if sol_file.exists() {
            args.push(sol_file.to_string_lossy().to_string());
        }
        if split_pdf && !pdf_on_left {
            if let Some(ref p) = actual_pdf {
                args.push(p.to_string_lossy().to_string());
            }
        }

        let win_candidates = [
            std::env::var("LOCALAPPDATA").ok().map(|l| std::path::PathBuf::from(l).join("Programs/Microsoft VS Code/bin/code.cmd")),
            std::env::var("ProgramFiles").ok().map(|p| std::path::PathBuf::from(p).join("Microsoft VS Code/bin/code.cmd")),
            std::env::var("ProgramFiles(x86)").ok().map(|p| std::path::PathBuf::from(p).join("Microsoft VS Code/bin/code.cmd")),
        ];

        for cand in win_candidates.into_iter().flatten() {
            if cand.exists() {
                let _ = std::process::Command::new(cand).args(&args).spawn();
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
        let candidates = [
            "/usr/local/bin/code",
            "/opt/homebrew/bin/code",
            "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code",
            "/usr/bin/code",
            "/usr/bin/codium",
        ];
        let code_bin = candidates
            .iter()
            .find(|p| std::path::Path::new(p).exists())
            .copied()
            .unwrap_or("code");

        let mut cmd = std::process::Command::new(code_bin);
        cmd.arg("--reuse-window");
        cmd.arg(target_dir);

        #[cfg(target_os = "macos")]
        {
            if split_pdf {
                if let Some(ref p) = actual_pdf {
                    let _ = std::process::Command::new("open").arg(p).spawn();
                }
            }
        }

        #[cfg(not(target_os = "macos"))]
        if split_pdf && pdf_on_left {
            if let Some(ref p) = actual_pdf {
                cmd.arg(p);
            }
        }

        if sol_file.exists() {
            cmd.arg(&sol_file);
        }

        #[cfg(not(target_os = "macos"))]
        if split_pdf && !pdf_on_left {
            if let Some(ref p) = actual_pdf {
                cmd.arg(p);
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(wayland) = std::env::var("WAYLAND_DISPLAY") {
                cmd.env("WAYLAND_DISPLAY", wayland);
            } else if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
                if std::path::Path::new(&runtime).join("wayland-0").exists() {
                    cmd.env("WAYLAND_DISPLAY", "wayland-0");
                }
            }
            if let Ok(disp) = std::env::var("DISPLAY") {
                cmd.env("DISPLAY", disp);
            } else {
                cmd.env("DISPLAY", ":0");
            }
            if let Ok(xauth) = std::env::var("XAUTHORITY") {
                cmd.env("XAUTHORITY", xauth);
            } else if let Some(home) = dirs::home_dir() {
                let default_xauth = home.join(".Xauthority");
                if default_xauth.exists() {
                    cmd.env("XAUTHORITY", default_xauth);
                }
            }
            if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
                cmd.env("XDG_RUNTIME_DIR", runtime);
            }
            if let Ok(dbus) = std::env::var("DBUS_SESSION_BUS_ADDRESS") {
                cmd.env("DBUS_SESSION_BUS_ADDRESS", dbus);
            }
        }

        let _ = cmd.spawn();

        #[cfg(target_os = "linux")]
        {
            std::thread::spawn(move || {
                for delay in [100, 350, 750] {
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                    activate_linux_window(&["code", "visual studio code", "codium"]);
                }
            });
        }

        #[cfg(target_os = "macos")]
        {
            let win_mode = macos_window_mode.to_string();
            if win_mode == "maximized" || win_mode == "fullscreen" {
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    let script = if win_mode == "fullscreen" {
                        r#"
                            tell application "Visual Studio Code" to activate
                            delay 0.1
                            tell application "System Events"
                                set codeProc to missing value
                                repeat with p in (get processes)
                                    if name of p is "Visual Studio Code" or name of p is "Code" then
                                        set codeProc to p
                                        exit repeat
                                    end if
                                end repeat
                                if codeProc is not missing value then
                                    tell codeProc
                                        if exists window 1 then
                                            try
                                                set value of attribute "AXFullScreen" of window 1 to true
                                            end try
                                        end if
                                    end tell
                                end if
                            end tell
                        "#
                    } else {
                        r#"
                            tell application "Visual Studio Code" to activate
                            delay 0.1
                            tell application "Finder"
                                set db to bounds of window of desktop
                                set screenW to item 3 of db
                                set screenH to item 4 of db
                            end tell
                            try
                                tell application "Visual Studio Code"
                                    set bounds of window 1 to {0, 25, screenW, screenH}
                                end tell
                            end try
                            tell application "System Events"
                                set codeProc to missing value
                                repeat with p in (get processes)
                                    if name of p is "Visual Studio Code" or name of p is "Code" then
                                        set codeProc to p
                                        exit repeat
                                    end if
                                end repeat
                                if codeProc is not missing value then
                                    tell codeProc
                                        if exists window 1 then
                                            try
                                                click (first button whose subrole is "AXZoomButton") of window 1
                                            end try
                                            try
                                                set position of window 1 to {0, 25}
                                                set size of window 1 to {screenW, screenH - 25}
                                            end try
                                        end if
                                    end tell
                                end if
                            end tell
                        "#
                    };
                    let _ = std::process::Command::new("osascript").args(["-e", script]).output();
                });
            }
        }
        let _ = macos_window_mode;
    }
}

fn get_candidate_bridge_sockets(problem_id: &str) -> Vec<std::path::PathBuf> {
    let mut list = Vec::new();
    let p_lower = problem_id.to_lowercase();
    let sock_names = [
        format!("algodeck_vscode_{}.sock", p_lower),
        "algodeck_vscode_algodeckworkspace.sock".to_string(),
        "algodeck_vscode_default.sock".to_string(),
    ];

    let mut base_dirs = Vec::new();
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        let pb = std::path::PathBuf::from(runtime);
        if pb.exists() {
            base_dirs.push(pb);
        }
    }
    base_dirs.push(std::path::PathBuf::from("/tmp"));
    if let Some(home) = dirs::home_dir() {
        base_dirs.push(home.join(".config").join("algodeck"));
    }

    // 1. Sprawdź aktywne wskaźniki (algodeck_vscode_active.sock)
    for b in &base_dirs {
        let active_file = b.join("algodeck_vscode_active.sock");
        if let Ok(content) = std::fs::read_to_string(&active_file) {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                let p = std::path::PathBuf::from(trimmed);
                if p.exists() && !list.contains(&p) {
                    list.push(p);
                }
            }
        }
    }

    // 2. Sprawdź znane nazwy socketów
    for b in &base_dirs {
        for name in &sock_names {
            let p = b.join(name);
            if p.exists() && !list.contains(&p) {
                list.push(p);
            }
        }
    }

    list
}

fn get_candidate_bridge_ports() -> Vec<u16> {
    let mut ports = Vec::new();
    let mut base_dirs = Vec::new();
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        base_dirs.push(std::path::PathBuf::from(runtime));
    }
    base_dirs.push(std::path::PathBuf::from("/tmp"));
    if let Some(home) = dirs::home_dir() {
        base_dirs.push(home.join(".config").join("algodeck"));
    }

    for b in &base_dirs {
        for fname in &["algodeck_bridge_port.txt", "bridge_port.txt"] {
            let p = b.join(fname);
            if let Ok(content) = std::fs::read_to_string(&p) {
                if let Ok(port) = content.trim().parse::<u16>() {
                    if port >= 49152 && port <= 49165 && !ports.contains(&port) {
                        ports.push(port);
                    }
                }
            }
        }
    }

    for p in 49152..=49157 {
        if !ports.contains(&p) {
            ports.push(p);
        }
    }

    ports
}

pub fn is_vscode_open_for_problem(problem_id: &str) -> bool {
    if problem_id.is_empty() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::io::{Write, Read};
        for sock_path in get_candidate_bridge_sockets(problem_id) {
            if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(&sock_path) {
                let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(80)));
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(80)));
                let req = format!("GET /ping?problem_id={} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n", url_encode(problem_id));
                if stream.write_all(req.as_bytes()).is_ok() {
                    let mut buf = [0u8; 1024];
                    if let Ok(n) = stream.read(&mut buf) {
                        if n > 0 && String::from_utf8_lossy(&buf[..n]).contains("200 OK") {
                            return true;
                        }
                    }
                }
            }
        }
    }

    for port in get_candidate_bridge_ports() {
        let ping_url = format!("http://127.0.0.1:{}/ping?problem_id={}", port, url_encode(problem_id));
        if let Ok(res) = ureq::get(&ping_url).timeout(std::time::Duration::from_millis(80)).call() {
            if res.status() == 200 {
                return true;
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("pgrep").args(["-f", "Visual Studio Code"]).output() {
            if output.status.success() && !output.stdout.is_empty() {
                return true;
            }
        }
        if let Ok(output) = std::process::Command::new("pgrep").args(["-x", "Code"]).output() {
            if output.status.success() && !output.stdout.is_empty() {
                return true;
            }
        }
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let is_proc_running = std::process::Command::new("pgrep").args(["-x", "code"]).output().map(|o| o.status.success() && !o.stdout.is_empty()).unwrap_or(false)
            || std::process::Command::new("pgrep").args(["-f", "code.*electron"]).output().map(|o| o.status.success() && !o.stdout.is_empty()).unwrap_or(false)
            || std::process::Command::new("pgrep").args(["-f", "/usr/lib64/vscode/code"]).output().map(|o| o.status.success() && !o.stdout.is_empty()).unwrap_or(false)
            || std::process::Command::new("pgrep").args(["-f", "com.visualstudio.code"]).output().map(|o| o.status.success() && !o.stdout.is_empty()).unwrap_or(false)
            || std::process::Command::new("pgrep").args(["-x", "codium"]).output().map(|o| o.status.success() && !o.stdout.is_empty()).unwrap_or(false);

        if is_proc_running {
            return true;
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("tasklist").args(["/FI", "IMAGENAME eq Code.exe"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("Code.exe") {
                return true;
            }
        }
    }
    false
}

pub fn send_bridge_open(_problem_id: &str, p_dir_str: &str, file_path_str: &str, vscode_mode: &str) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        use std::io::{Write, Read};
        for sock_path in get_candidate_bridge_sockets(_problem_id) {
            if let Ok(mut stream) = UnixStream::connect(&sock_path) {
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(120)));
                let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(120)));
                let req = format!(
                    "POST /open?pdir={}&file={}&mode={} HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    url_encode(p_dir_str),
                    url_encode(file_path_str),
                    url_encode(vscode_mode)
                );
                if stream.write_all(req.as_bytes()).is_ok() {
                    let mut buf = [0u8; 1024];
                    if let Ok(n) = stream.read(&mut buf) {
                        if n > 0 && String::from_utf8_lossy(&buf[..n]).contains("200 OK") {
                            return true;
                        }
                    }
                }
            }
        }
    }

    for port in get_candidate_bridge_ports() {
        let bridge_url = format!("http://127.0.0.1:{}/open?pdir={}&file={}&mode={}", port, url_encode(p_dir_str), url_encode(file_path_str), url_encode(vscode_mode));
        if let Ok(res) = ureq::post(&bridge_url).timeout(std::time::Duration::from_millis(120)).call() {
            if res.status() == 200 {
                return true;
            }
        }
    }

    false
}

pub fn send_bridge_request(endpoint: &str, _problem_id: &str, p_dir_str: &str) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        use std::io::{Write, Read};
        for sock_path in get_candidate_bridge_sockets(_problem_id) {
            if let Ok(mut stream) = UnixStream::connect(&sock_path) {
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(120)));
                let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(120)));
                let req = format!(
                    "POST /{}?pdir={} HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    endpoint,
                    url_encode(p_dir_str)
                );
                if stream.write_all(req.as_bytes()).is_ok() {
                    let mut buf = [0u8; 1024];
                    if let Ok(n) = stream.read(&mut buf) {
                        if n > 0 && String::from_utf8_lossy(&buf[..n]).contains("200 OK") {
                            return true;
                        }
                    }
                }
            }
        }
    }

    for port in get_candidate_bridge_ports() {
        let bridge_url = format!("http://127.0.0.1:{}/{}?pdir={}", port, endpoint, url_encode(p_dir_str));
        if let Ok(res) = ureq::post(&bridge_url).timeout(std::time::Duration::from_millis(120)).call() {
            if res.status() == 200 {
                return true;
            }
        }
    }

    false
}

pub fn spawn_in_system_terminal(endpoint: &str, p_dir: &std::path::Path, _problem_id: &str) {
    #[cfg(target_os = "linux")]
    {
        let script_name = if endpoint == "test" { "test.sh" } else { "run.sh" };
        let script_path = p_dir.join(".algo").join(script_name);
        let script_str = script_path.to_string_lossy();
        let cmd = format!("cd '{}' && bash '{}'; echo ''; read -p 'Naciśnij Enter, aby zamknąć...' -r; exit", p_dir.display(), script_str);

        let p_dir_str = p_dir.to_string_lossy().to_string();
        let terminals = [
            ("konsole", vec!["--workdir", &p_dir_str, "-e", "bash", "-c", &cmd]),
            ("gnome-terminal", vec!["--working-directory", &p_dir_str, "--", "bash", "-c", &cmd]),
            ("xfce4-terminal", vec!["--working-directory", &p_dir_str, "-e", &cmd]),
            ("kitty", vec!["--directory", &p_dir_str, "bash", "-c", &cmd]),
            ("alacritty", vec!["--working-directory", &p_dir_str, "-e", "bash", "-c", &cmd]),
            ("x-terminal-emulator", vec!["-e", "bash", "-c", &cmd]),
            ("xterm", vec!["-e", "bash", "-c", &cmd]),
        ];

        for (term_bin, args) in terminals {
            let mut proc = std::process::Command::new(term_bin);
            proc.args(&args);
            if let Ok(wayland) = std::env::var("WAYLAND_DISPLAY") {
                proc.env("WAYLAND_DISPLAY", wayland);
            } else if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
                if std::path::Path::new(&runtime).join("wayland-0").exists() {
                    proc.env("WAYLAND_DISPLAY", "wayland-0");
                }
            }
            if let Ok(disp) = std::env::var("DISPLAY") {
                proc.env("DISPLAY", disp);
            }
            if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
                proc.env("XDG_RUNTIME_DIR", runtime);
            }
            if let Ok(dbus) = std::env::var("DBUS_SESSION_BUS_ADDRESS") {
                proc.env("DBUS_SESSION_BUS_ADDRESS", dbus);
            }
            if proc.spawn().is_ok() {
                println!("🚀 AlgoDeck: Uruchomiono {} w terminalu {}", script_name, term_bin);
                return;
            }
        }

        let _ = std::process::Command::new("bash").arg(&script_path).current_dir(p_dir).spawn();
    }

    #[cfg(target_os = "macos")]
    {
        let script_name = if endpoint == "test" { "test.sh" } else { "run.sh" };
        let script_path = p_dir.join(".algo").join(script_name);
        let script_str = script_path.to_string_lossy();
        let osa_cmd = format!(
            "tell application \"Terminal\" to do script \"cd '{}' && bash '{}'\"",
            p_dir.display(),
            script_str
        );
        let _ = std::process::Command::new("osascript").args(["-e", &osa_cmd]).spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let cmd_script = if endpoint == "test" { "test.cmd" } else { "run.cmd" };
        let cmd_path = p_dir.join(".algo").join(cmd_script);
        let mut cmd = std::process::Command::new("cmd.exe");
        cmd.args(["/C", "start", "cmd.exe", "/K", &format!("cd /d \"{}\" && \"{}\"", p_dir.display(), cmd_path.display())]);
        let _ = cmd.spawn();
    }
}

pub fn trigger_vscode_action<F>(endpoint: &str, problem_id: &str, ws_dir: &std::path::Path, open_vscode: F) -> bool
where
    F: Fn(&str),
{
    let p_dir = WorkspaceManager::get_problem_dir(ws_dir, problem_id);
    let p_dir_str = p_dir.to_string_lossy().to_string();
    let upper_pid = problem_id.to_uppercase();
    let _ = std::fs::write(std::env::temp_dir().join("algodeck_active_task.txt"), &upper_pid);
    #[cfg(unix)]
    let _ = std::fs::write("/tmp/algodeck_active_task.txt", &upper_pid);

    // 1. ZAPISZ TRIGGER FILE (File IPC) - natychmiastowe, odporne na SELinux, sandbox, flatpak, sieć
    let algo_dir = p_dir.join(".algo");
    let _ = std::fs::create_dir_all(&algo_dir);
    let trigger_algo = algo_dir.join("trigger.json");
    let trigger_root = p_dir.join("algodeck_trigger.json");
    let ack_algo = algo_dir.join("trigger_ack.json");
    let ack_root = p_dir.join("trigger_ack.json");

    // Usuń stare potwierdzenia ACK
    let _ = std::fs::remove_file(&ack_algo);
    let _ = std::fs::remove_file(&ack_root);

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let trigger_payload = serde_json::json!({
        "action": endpoint,
        "pdir": p_dir_str,
        "problem_id": problem_id,
        "timestamp": now_ts
    }).to_string();

    let _ = std::fs::write(&trigger_algo, &trigger_payload);
    let _ = std::fs::write(&trigger_root, &trigger_payload);

    // 2. Spróbuj wysłać żądanie przez bezpośredni socket UNIX lub serwer HTTP (jeśli aktywny)
    if send_bridge_request(endpoint, problem_id, &p_dir_str) {
        let _ = std::fs::remove_file(&trigger_algo);
        let _ = std::fs::remove_file(&trigger_root);
        return true;
    }

    // 3. Sprawdź czy VS Code odebrał trigger przez File Watcher (krótki poll do 250ms)
    for _ in 0..5 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let consumed = !trigger_algo.exists() || !trigger_root.exists() || ack_algo.exists() || ack_root.exists();
        if consumed {
            let _ = std::fs::remove_file(&trigger_algo);
            let _ = std::fs::remove_file(&trigger_root);
            let _ = std::fs::remove_file(&ack_algo);
            let _ = std::fs::remove_file(&ack_root);
            return true;
        }
        if send_bridge_request(endpoint, problem_id, &p_dir_str) {
            let _ = std::fs::remove_file(&trigger_algo);
            let _ = std::fs::remove_file(&trigger_root);
            return true;
        }
    }

    // 4. Jeśli okno VS Code nie jest otwarte dla tego zadania, otwórz je
    let was_open = is_vscode_open_for_problem(problem_id);
    if !was_open {
        open_vscode(problem_id);
        // Poczekaj chwilę na start okna i File Watchera w VS Code
        for _ in 0..6 {
            std::thread::sleep(std::time::Duration::from_millis(70));
            let consumed = !trigger_algo.exists() || !trigger_root.exists() || ack_algo.exists() || ack_root.exists();
            if consumed {
                let _ = std::fs::remove_file(&trigger_algo);
                let _ = std::fs::remove_file(&trigger_root);
                let _ = std::fs::remove_file(&ack_algo);
                let _ = std::fs::remove_file(&ack_root);
                return true;
            }
            if send_bridge_request(endpoint, problem_id, &p_dir_str) {
                let _ = std::fs::remove_file(&trigger_algo);
                let _ = std::fs::remove_file(&trigger_root);
                return true;
            }
        }
    }

    // Posprzątaj triggery przed uruchomieniem w terminalu systemowym
    let _ = std::fs::remove_file(&trigger_algo);
    let _ = std::fs::remove_file(&trigger_root);

    // 5. Niezawodny Fallback: Uruchomienie w natywnym oknie terminala systemowego (KDE Konsole, GNOME, itp.)
    spawn_in_system_terminal(endpoint, &p_dir, problem_id);
    true
}

pub fn ensure_vscode_open_for_problem(problem_id: &str, state: &AppState) {
    if problem_id.is_empty() {
        return;
    }
    if !is_vscode_open_for_problem(problem_id) {
        let ws_dir = state.settings.read().unwrap().get_workspace_path();
        let (split_pdf, pdf_pos, split_ratio, vscode_mode, macos_mode) = {
            let s = state.settings.read().unwrap();
            (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone(), s.macos_window_mode.clone())
        };
        open_in_vscode_internal(problem_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode, &macos_mode);
    }
}

pub fn safe_kill_problem_process(problem_id: &str, ws_dir: &std::path::Path) {
    if problem_id.is_empty() {
        return;
    }
    let p_dir = WorkspaceManager::get_problem_dir(ws_dir, problem_id);
    let kill_script = p_dir.join(".algo").join("kill.sh");
    if kill_script.exists() {
        let _ = std::process::Command::new("bash").arg(kill_script).output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Dokładna nazwa pliku binarnego (nigdy -f ze słowem kluczowym, bo to ubiłoby VS Code!)
        let _ = std::process::Command::new("pkill").arg("-x").arg(problem_id).output();
        let _ = std::process::Command::new("pkill").arg("-f").arg(format!("{}/\\.algo/run\\.sh", problem_id)).output();
        let _ = std::process::Command::new("pkill").arg("-f").arg(format!("{}/\\.algo/test\\.sh", problem_id)).output();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill").args(["/F", "/IM", &format!("{}.exe", problem_id)]).output();
    }
}

async fn switch_task(
    State(state): State<AppState>,
    AxumPath(direction): AxumPath<String>,
) -> Json<serde_json::Value> {
    let ws_dir = state.settings.read().unwrap().get_workspace_path();
    let problems = WorkspaceManager::list_problems(&ws_dir);
    if problems.is_empty() {
        return Json(serde_json::json!({ "success": false }));
    }

    let current = state.streamdeck.get_state().active_problem_id.unwrap_or_default();
    let mut idx = 0;
    for (i, p) in problems.iter().enumerate() {
        if p.problem_id == current {
            idx = i;
            break;
        }
    }

    let next_idx = if direction == "next" {
        (idx + 1) % problems.len()
    } else {
        if idx == 0 { problems.len() - 1 } else { idx - 1 }
    };

    let next_id = &problems[next_idx].problem_id;
    state.streamdeck.set_active_problem_with_meta(next_id, next_idx + 1, problems.len());
    let _ = state.ws_sender.send(format!("active:{}", next_id));

    let (split_pdf, pdf_pos, split_ratio, vscode_mode, macos_mode) = {
        let s = state.settings.read().unwrap();
        (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone(), s.macos_window_mode.clone())
    };
    open_in_vscode_internal(next_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode, &macos_mode);

    Json(serde_json::json!({ "success": true, "problem_id": next_id }))
}

async fn compile_task(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Json<CompileResult> {
    ensure_vscode_open_for_problem(&id, &state);
    state.streamdeck.update_key_status(5, "RUNNING", Some("COMPILING..."));
    let res = state.runner.compile(&id, false);
    if res.success {
        state.streamdeck.update_key_status(5, "ACTION", Some(&format!("{:.0}ms", res.duration_ms)));
    } else {
        state.streamdeck.update_key_status(5, "CE", Some("ERROR"));
    }
    Json(res)
}

async fn run_all_task(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Json<RunAllSummary> {
    ensure_vscode_open_for_problem(&id, &state);
    state.streamdeck.update_key_status(6, "RUNNING", Some("TESTING..."));
    let res = state.runner.run_all_tests(&id);
    if res.all_passed {
        state.streamdeck.update_key_status(6, "OK", Some(&format!("{:.0}ms", res.total_time_ms)));
    } else {
        state.streamdeck.update_key_status(6, "WA", Some(&format!("{}/{}", res.passed_count, res.total_tests)));
    }
    Json(res)
}

async fn run_single_test_task(
    State(state): State<AppState>,
    AxumPath((id, test_id)): AxumPath<(String, String)>,
) -> Json<TestResult> {
    let res = state.runner.run_single_test(&id, &test_id, false);
    Json(res)
}

async fn kill_task(
    State(state): State<AppState>,
    AxumPath(_id): AxumPath<String>,
) -> Json<serde_json::Value> {
    ensure_vscode_open_for_problem(&_id, &state);
    let ws_dir = state.settings.read().unwrap().get_workspace_path();
    safe_kill_problem_process(&_id, &ws_dir);
    state.streamdeck.update_key_status(8, "DANGER", Some("KILLED"));
    Json(serde_json::json!({ "success": true }))
}

#[derive(Deserialize, Default)]
pub struct LaunchRequest {
    pub problem_id: Option<String>,
}

async fn launch_screen(
    State(state): State<AppState>,
    req: Option<Json<LaunchRequest>>,
) -> Json<serde_json::Value> {
    let pid = req.and_then(|r| r.problem_id.clone()).unwrap_or_else(|| "test".to_string());
    state.streamdeck.set_active_problem_with_meta(&pid, 1, 1);
    Json(serde_json::json!({ "success": true, "problem_id": pid }))
}

async fn exit_screen(State(state): State<AppState>) -> Json<serde_json::Value> {
    state.streamdeck.set_idle_screen();
    Json(serde_json::json!({ "success": true }))
}

async fn shutdown_server(State(state): State<AppState>) -> Json<serde_json::Value> {
    state.shutdown_signal.store(true, Ordering::Relaxed);
    state.streamdeck.stop();
    tokio::task::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(80));
        StreamDeckEngine::blank_device_if_connected();
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("systemctl").args(["--user", "stop", "algodeck.service"]).output();
        }
        #[cfg(target_os = "macos")]
        {
            let uid = unsafe { libc::getuid() };
            let service_target = format!("gui/{}/com.algodeck.daemon", uid);
            let _ = std::process::Command::new("launchctl").args(["bootout", &service_target]).output();
            if let Some(home) = dirs::home_dir() {
                let plist = home.join("Library/LaunchAgents/com.algodeck.daemon.plist");
                if plist.exists() {
                    let _ = std::process::Command::new("launchctl")
                        .args(["unload"])
                        .arg(&plist)
                        .output();
                }
            }
        }
    });
    Json(serde_json::json!({ "success": true }))
}

async fn get_streamdeck_state(State(state): State<AppState>) -> Json<StreamDeckState> {
    Json(state.streamdeck.get_state())
}

async fn press_streamdeck_key(
    State(state): State<AppState>,
    AxumPath(key_id): AxumPath<usize>,
) -> Json<serde_json::Value> {
    let action = {
        let sd_state = state.streamdeck.get_state();
        if key_id < sd_state.keys.len() {
            sd_state.keys[key_id].action.clone()
        } else {
            String::new()
        }
    };

    if !action.is_empty() {
        let active_id = state.streamdeck.get_state().active_problem_id.unwrap_or_default();
        let ws_dir = state.settings.read().unwrap().get_workspace_path();

        if action.starts_with("select_task:") {
            let pid = action.trim_start_matches("select_task:").to_string();
            let problems = WorkspaceManager::list_problems(&ws_dir);
            let cur_idx = problems.iter().position(|p| p.problem_id == pid).unwrap_or(0) + 1;
            state.streamdeck.set_active_problem_with_meta(&pid, cur_idx, problems.len().max(1));
            let (split_pdf, pdf_pos, split_ratio, vscode_mode, macos_mode) = {
                let s = state.settings.read().unwrap();
                (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone(), s.macos_window_mode.clone())
            };
            open_in_vscode_internal(&pid, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode, &macos_mode);
            return Json(serde_json::json!({ "success": true, "action": action }));
        }

        match action.as_str() {
            "open_vscode" => {
                let (split_pdf, pdf_pos, split_ratio, vscode_mode, macos_mode) = {
                    let s = state.settings.read().unwrap();
                    (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone(), s.macos_window_mode.clone())
                };
                open_in_vscode_internal(&active_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode, &macos_mode);
            }
            "run_all" => {
                if !active_id.is_empty() {
                    let ws = ws_dir.clone();
                    let pid = active_id.clone();
                    let state_c = state.clone();
                    tokio::task::spawn_blocking(move || {
                        let open_fn = |p: &str| ensure_vscode_open_for_problem(p, &state_c);
                        trigger_vscode_action("test", &pid, &ws, open_fn);
                        let _ = state_c.runner.run_all_tests(&pid);
                    });
                }
            }
            "run_interactive" => {
                if !active_id.is_empty() {
                    let ws = ws_dir.clone();
                    let pid = active_id.clone();
                    let state_c = state.clone();
                    tokio::task::spawn_blocking(move || {
                        let open_fn = |p: &str| ensure_vscode_open_for_problem(p, &state_c);
                        trigger_vscode_action("run", &pid, &ws, open_fn);
                    });
                }
            }
            "kill_process" => {
                if !active_id.is_empty() {
                    ensure_vscode_open_for_problem(&active_id, &state);
                    safe_kill_problem_process(&active_id, &ws_dir);
                    state.streamdeck.update_key_status(8, "DANGER", Some("KILLED"));
                }
            }
            "new_task" => {
                open_browser_foreground("http://127.0.0.1:8080/popup.html?mode=tab&trigger=pdf");
            }
            "open_settings" => {
                open_browser_foreground("http://127.0.0.1:8080/settings");
            }
            "open_menu" => {
                let problems = WorkspaceManager::list_problems(&ws_dir);
                let pids: Vec<String> = problems.into_iter().map(|p| p.problem_id).collect();
                state.streamdeck.set_menu_screen(&pids, 0);
            }
            "back_to_task" => {
                if !active_id.is_empty() {
                    let problems = WorkspaceManager::list_problems(&ws_dir);
                    let cur_idx = problems.iter().position(|p| p.problem_id == active_id).unwrap_or(0) + 1;
                    state.streamdeck.set_active_problem_with_meta(&active_id, cur_idx, problems.len().max(1));
                } else {
                    state.streamdeck.set_idle_screen();
                }
            }
            "exit_to_idle" => {
                state.streamdeck.set_idle_screen();
            }
            "start_or_resume" => {
                let problems = WorkspaceManager::list_problems(&ws_dir);
                if let Some(first) = problems.first() {
                    state.streamdeck.set_active_problem_with_meta(&first.problem_id, 1, problems.len().max(1));
                    let (split_pdf, pdf_pos, split_ratio, vscode_mode, macos_mode) = {
                        let s = state.settings.read().unwrap();
                        (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone(), s.macos_window_mode.clone())
                    };
                    open_in_vscode_internal(&first.problem_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode, &macos_mode);
                } else {
                    open_browser_foreground("http://127.0.0.1:8080/");
                }
            }
            "prev_task" => {
                let _ = switch_task(State(state.clone()), AxumPath("prev".to_string())).await;
            }
            "next_task" => {
                let _ = switch_task(State(state.clone()), AxumPath("next".to_string())).await;
            }
            _ => {}
        }
    }

    Json(serde_json::json!({ "success": true, "action": action }))
}

// ----------------- WebSocket Handler -----------------

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.ws_sender.subscribe();

    // Send initial state
    let init_json = serde_json::to_string(&state.streamdeck.get_state()).unwrap_or_default();
    let _ = socket.send(Message::Text(init_json)).await;

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(t))) => {
                        // Echo or handle client message
                        if t == "ping" {
                            let _ = socket.send(Message::Text("pong".to_string())).await;
                        }
                    }
                    _ => break,
                }
            }
            broadcast_msg = rx.recv() => {
                if let Ok(b) = broadcast_msg {
                    if socket.send(Message::Text(b)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

pub fn start_detached_daemon() {
    match ureq::get("http://127.0.0.1:8080/api/status").timeout(std::time::Duration::from_millis(500)).call() {
        Ok(_) => {
            println!("⚡ AlgoDeck już działa w tle na porcie 8080.");
            return;
        }
        Err(_) => {}
    }

    crate::streamdeck::kill_streamdeck_conflicts();
    crate::native_host::ensure_native_manifests();

    #[cfg(target_os = "linux")]
    {
        let status = std::process::Command::new("systemctl")
            .args(["--user", "start", "algodeck.service"])
            .status();
        if let Ok(s) = status {
            if s.success() {
                for _ in 0..10 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if ureq::get("http://127.0.0.1:8080/api/status").timeout(std::time::Duration::from_millis(300)).call().is_ok() {
                        println!("✓ Usługa algodeck.service została uruchomiona przez systemd!");
                        return;
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let uid = unsafe { libc::getuid() };
        let service_target = format!("gui/{}/com.algodeck.daemon", uid);

        if let Some(home) = dirs::home_dir() {
            let plist = home.join("Library/LaunchAgents/com.algodeck.daemon.plist");
            if plist.exists() {
                let _ = std::process::Command::new("launchctl")
                    .args(["enable", &service_target])
                    .output();
                let _ = std::process::Command::new("launchctl")
                    .args(["bootstrap", &format!("gui/{}", uid)])
                    .arg(&plist)
                    .output();
                let _ = std::process::Command::new("launchctl")
                    .args(["load"])
                    .arg(&plist)
                    .output();
                let _ = std::process::Command::new("launchctl")
                    .args(["kickstart", "-k", &service_target])
                    .output();
                let _ = std::process::Command::new("launchctl")
                    .args(["start", "com.algodeck.daemon"])
                    .output();
            }
        }

        for _ in 0..15 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if ureq::get("http://127.0.0.1:8080/api/status").timeout(std::time::Duration::from_millis(300)).call().is_ok() {
                println!("✓ Usługa LaunchAgent (com.algodeck.daemon) została pomyślnie uruchomiona przez launchctl!");
                return;
            }
        }
    }

    println!("⚡ Uruchamianie daemona AlgoDeck w tle...");
    let self_exe = std::env::current_exe().unwrap_or_else(|_| {
        dirs::home_dir()
            .map(|h| h.join(".local/bin/algodeck"))
            .unwrap_or_else(|| std::path::PathBuf::from("algodeck"))
    });

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let mut cmd = std::process::Command::new(&self_exe);
        cmd.arg("daemon");
        cmd.stdin(std::process::Stdio::null());

        let log_dir = dirs::home_dir()
            .map(|h| h.join(".local/share/algodeck"))
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        let _ = std::fs::create_dir_all(&log_dir);
        let log_path = log_dir.join("daemon.log");

        if let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
            if let Ok(err_file) = file.try_clone() {
                cmd.stdout(file);
                cmd.stderr(err_file);
            }
        }

        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }

        match cmd.spawn() {
            Ok(_) => {
                for _ in 0..20 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if ureq::get("http://127.0.0.1:8080/api/status").timeout(std::time::Duration::from_millis(300)).call().is_ok() {
                        println!("✓ Daemon AlgoDeck został pomyślnie uruchomiony w tle!");
                        return;
                    }
                }
                println!("✓ Polecenie startu daemona wysłane. Sprawdź status poleceniem 'algodeck status'.");
            }
            Err(e) => {
                eprintln!("Błąd startu daemona: {}", e);
            }
        }
    }

    #[cfg(not(unix))]
    {
        match std::process::Command::new(self_exe).arg("daemon").spawn() {
            Ok(_) => {
                println!("✓ Daemon został pomyślnie uruchomiony w tle!");
            }
            Err(e) => {
                eprintln!("Błąd startu daemona: {}", e);
            }
        }
    }
}

pub fn stop_daemon() {
    println!("⏹ Zatrzymywanie AlgoDeck...");
    let _ = ureq::post("http://127.0.0.1:8080/api/shutdown").timeout(std::time::Duration::from_millis(800)).call();
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("systemctl").args(["--user", "stop", "algodeck.service"]).output();
        let _ = std::process::Command::new("pkill").arg("-f").arg("algodeck daemon").output();
    }
    #[cfg(target_os = "macos")]
    {
        let uid = unsafe { libc::getuid() };
        let service_target = format!("gui/{}/com.algodeck.daemon", uid);
        let _ = std::process::Command::new("launchctl").args(["bootout", &service_target]).output();
        if let Some(home) = dirs::home_dir() {
            let plist = home.join("Library/LaunchAgents/com.algodeck.daemon.plist");
            if plist.exists() {
                let _ = std::process::Command::new("launchctl")
                    .args(["unload"])
                    .arg(&plist)
                    .output();
            }
        }
        let _ = std::process::Command::new("pkill").arg("-f").arg("algodeck daemon").output();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill").args(["/F", "/IM", "algodeck.exe"]).output();
    }
    std::thread::sleep(std::time::Duration::from_millis(150));
    crate::streamdeck::StreamDeckEngine::blank_device_if_connected();
    println!("✓ AlgoDeck zatrzymany, ekran Stream Decka wygaszony.");
}

