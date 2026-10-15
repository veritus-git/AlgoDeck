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

#[derive(Deserialize)]
pub struct ManualTaskRequest {
    pub problem_id: String,
    pub title: String,
    pub time_limit: f64,
    pub memory_limit: u64,
    #[serde(default)]
    pub sample_cin: String,
    #[serde(default)]
    pub sample_cout: String,
}

#[derive(Deserialize, Default)]
pub struct ClearStagedRequest {
    pub file_type: Option<String>,
}

pub fn create_router(app_state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/status", get(get_status))
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
    Json(StatusResponse {
        status: "ok".to_string(),
        version: "2.0.0".to_string(),
        active_problem: sd_state.active_problem_id,
        workspace_dir: settings.workspace_dir,
        streamdeck_connected: sd_state.is_hardware_connected,
    })
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
    let _ = new_settings.save();
    {
        let mut w = state.settings.write().unwrap();
        *w = new_settings;
    }
    Ok(Json(serde_json::json!({ "success": true })))
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
            if enable {
                let _ = std::process::Command::new("launchctl").args(["load", "-w"]).arg(&plist).output();
            } else {
                let _ = std::process::Command::new("launchctl").args(["unload", "-w"]).arg(&plist).output();
            }
        }
    }
}

async fn open_gui() -> Json<serde_json::Value> {
    let _ = open::that("http://127.0.0.1:8080/");
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
                let (split_pdf, pdf_pos, split_ratio, vscode_mode) = {
                    let s = state.settings.read().unwrap();
                    (s.open_pdf_in_vscode && is_pdf, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone())
                };
                open_in_vscode_internal(&analysis.problem_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode);
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
    if !req.sample_cin.trim().is_empty() {
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
            let (split_pdf, pdf_pos, split_ratio, vscode_mode) = {
                let s = state.settings.read().unwrap();
                (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone())
            };
            open_in_vscode_internal(&pid, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode);
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

async fn stage_by_path(
    Json(req): Json<StagePathRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut path = PathBuf::from(&req.file_path);
    if !path.exists() {
        if let Some(fname) = path.file_name() {
            if let Some(dl) = dirs::download_dir() {
                let candidate = dl.join(fname);
                if candidate.exists() {
                    path = candidate;
                }
            }
        }
    }

    if !path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "success": false, "detail": "Plik nie istnieje na dysku" })),
        ));
    }

    let is_pdf = path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("pdf")).unwrap_or(false);
    let is_zip = path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("zip")).unwrap_or(false);

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

    let filename = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "downloaded_file".to_string());
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
            let (split_pdf, pdf_pos, split_ratio, vscode_mode) = {
                let s = state.settings.read().unwrap();
                (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone())
            };
            open_in_vscode_internal(&analysis.problem_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode);
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
        let (split_pdf, pdf_pos, split_ratio, vscode_mode) = {
            let s = state.settings.read().unwrap();
            (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone())
        };
        open_in_vscode_internal(&analysis.problem_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode);
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

    let (split_pdf, pdf_pos, split_ratio, vscode_mode) = {
        let s = state.settings.read().unwrap();
        (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone())
    };
    open_in_vscode_internal(&id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode);
    Json(serde_json::json!({ "success": true }))
}

async fn stage_refresh() -> Json<serde_json::Value> {
    get_staged_state().await
}

pub fn open_in_vscode_internal(
    problem_id: &str,
    workspace_dir: &Path,
    split_pdf: bool,
    pdf_position: &str,
    split_ratio: u8,
    vscode_mode: &str,
) {
    static LAST_VSCODE_LAUNCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let last = LAST_VSCODE_LAUNCH.swap(now, std::sync::atomic::Ordering::Relaxed);
    if now.saturating_sub(last) < 1000 {
        return;
    }

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

    // Upewnij się, że zadanie posiada skrypty i .vscode/tasks.json
    WorkspaceManager::ensure_problem_scripts(workspace_dir, problem_id, problem_id, 1.0);

    let pdf_on_left = pdf_position == "left";
    let is_separate = vscode_mode == "separate_windows";
    let target_dir = if is_separate { &p_dir } else { workspace_dir };

    #[cfg(target_os = "windows")]
    {
        let mut args = Vec::new();
        if is_separate {
            args.push("--new-window".to_string());
        } else {
            args.push("--reuse-window".to_string());
        }
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
        let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into());
        let xauthority = std::env::var("XAUTHORITY").unwrap_or_else(|_| "/home/linux/.Xauthority".into());

        let sd_code = dirs::home_dir().map(|h| h.join(".local/bin/sd_algo_code.sh"));
        let mut launched = false;
        if let Some(ref script) = sd_code {
            if script.exists() {
                let res = std::process::Command::new(script)
                    .arg(&p_dir)
                    .arg(&sol_file)
                    .env("DISPLAY", &display)
                    .env("XAUTHORITY", &xauthority)
                    .env("ALGODECK_PDF_POS", pdf_position)
                    .env("ALGODECK_SPLIT_RATIO", split_ratio.to_string())
                    .env("ALGODECK_VSCODE_MODE", vscode_mode)
                    .env("ALGODECK_WORKSPACE_DIR", workspace_dir.to_string_lossy().as_ref())
                    .spawn();
                if res.is_ok() {
                    launched = true;
                }
            }
        }

        if !launched {
            let candidates = [
                "/usr/local/bin/code",
                "/opt/homebrew/bin/code",
                "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code",
                "/usr/bin/code",
            ];
            let code_bin = candidates
                .iter()
                .find(|p| std::path::Path::new(p).exists())
                .copied()
                .unwrap_or("code");

            let mut cmd = std::process::Command::new(code_bin);
            if is_separate {
                cmd.arg("--new-window");
            } else {
                cmd.arg("--reuse-window");
            }
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
                cmd.env("DISPLAY", &display);
                cmd.env("XAUTHORITY", &xauthority);
            }
            let _ = cmd.spawn();
        }

        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("notify-send")
                .args(["AlgoDeck", &format!("Otwarto zadanie {} w Visual Studio Code", problem_id.to_uppercase())])
                .env("DISPLAY", &display)
                .env("XAUTHORITY", &xauthority)
                .spawn();
        }
    }
}

pub fn is_vscode_open_for_problem(problem_id: &str) -> bool {
    if problem_id.is_empty() {
        return false;
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = std::process::Command::new("wmctrl").arg("-l").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line_lower = line.to_lowercase();
                if (line_lower.contains("visual studio code") || line_lower.contains(" - code"))
                    && line_lower.contains(&problem_id.to_lowercase())
                {
                    return true;
                }
            }
        }
        if let Ok(output) = std::process::Command::new("pgrep").args(["-f", &format!("code.*{}", problem_id)]).output() {
            if output.status.success() && !output.stdout.is_empty() {
                return true;
            }
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

pub fn ensure_vscode_open_for_problem(problem_id: &str, state: &AppState) {
    if problem_id.is_empty() {
        return;
    }
    if !is_vscode_open_for_problem(problem_id) {
        let ws_dir = state.settings.read().unwrap().get_workspace_path();
        let (split_pdf, pdf_pos, split_ratio, vscode_mode) = {
            let s = state.settings.read().unwrap();
            (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone())
        };
        open_in_vscode_internal(problem_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode);
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
            if let Some(home) = dirs::home_dir() {
                let plist = home.join("Library/LaunchAgents/com.algodeck.daemon.plist");
                if plist.exists() {
                    let _ = std::process::Command::new("launchctl")
                        .args(["unload", "-w"])
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
            let (split_pdf, pdf_pos, split_ratio, vscode_mode) = {
                let s = state.settings.read().unwrap();
                (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone())
            };
            open_in_vscode_internal(&pid, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode);
            return Json(serde_json::json!({ "success": true, "action": action }));
        }

        match action.as_str() {
            "open_vscode" => {
                let (split_pdf, pdf_pos, split_ratio, vscode_mode) = {
                    let s = state.settings.read().unwrap();
                    (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone())
                };
                open_in_vscode_internal(&active_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode);
            }
            "run_all" => {
                ensure_vscode_open_for_problem(&active_id, &state);
                let _ = state.runner.run_all_tests(&active_id);
            }
            "run_interactive" => {
                ensure_vscode_open_for_problem(&active_id, &state);
                #[cfg(not(target_os = "windows"))]
                {
                    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into());
                    let _ = std::process::Command::new("x-terminal-emulator")
                        .args(["-e", &format!("algodeck run {}", active_id)])
                        .env("DISPLAY", &display)
                        .spawn();
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
                let _ = open::that("http://127.0.0.1:8080/#new");
            }
            "open_settings" => {
                let _ = open::that("http://127.0.0.1:8080/settings");
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
                    let (split_pdf, pdf_pos, split_ratio, vscode_mode) = {
                        let s = state.settings.read().unwrap();
                        (s.open_pdf_in_vscode, s.pdf_position.clone(), s.editor_split_ratio, s.vscode_mode.clone())
                    };
                    open_in_vscode_internal(&first.problem_id, &ws_dir, split_pdf, &pdf_pos, split_ratio, &vscode_mode);
                } else {
                    let _ = open::that("http://127.0.0.1:8080/");
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
