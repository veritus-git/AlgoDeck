use crate::config::AppSettings;
use crate::pdf::{PdfParser, ProblemAnalysis, TestCase};
use crate::runner::{CompileResult, RunAllSummary, TestExecutor, TestResult};
use crate::streamdeck::{StreamDeckEngine, StreamDeckState};
use crate::workspace::{ProblemManifest, ProblemSummary, WorkspaceManager};

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

#[derive(Deserialize)]
pub struct StageFileRequest {
    pub file_path: String,
    #[allow(dead_code)]
    pub file_type: String,
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
        .route("/upload-staging", post(upload_staging))
        .route("/clear-staged", post(clear_staged))
        .route("/start-staged", post(start_staged))
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
        .route("/streamdeck/press/:key_id", post(press_streamdeck_key));

    let mut router = Router::new()
        .nest("/api", api_routes)
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // Serve static frontend if directory exists
    let frontend_dir = PathBuf::from("./frontend");
    if frontend_dir.exists() {
        router = router.fallback_service(ServeDir::new(frontend_dir));
    }

    router
}

// ----------------- Handlers -----------------

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
    let _ = new_settings.save();
    {
        let mut w = state.settings.write().unwrap();
        *w = new_settings;
    }
    Ok(Json(serde_json::json!({ "success": true })))
}

async fn open_gui() -> Json<serde_json::Value> {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg("algodeck || ~/.local/bin/algodeck || target/release/algodeck &")
            .spawn();
    }
    Json(serde_json::json!({ "success": true }))
}

async fn get_problems(State(state): State<AppState>) -> Json<Vec<ProblemSummary>> {
    let ws_dir = state.settings.read().unwrap().get_workspace_path();
    let list = WorkspaceManager::list_problems(&ws_dir);
    Json(list)
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
    let path = Path::new(&req.file_path);
    if !path.exists() {
        return Json(serde_json::json!({ "success": false, "detail": "Plik PDF nie istnieje" }));
    }

    let text = match PdfParser::extract_text_from_file(path) {
        Ok(t) => t,
        Err(e) => return Json(serde_json::json!({ "success": false, "detail": e })),
    };

    let mut analysis = PdfParser::parse_heuristics(&text);
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
        match WorkspaceManager::create_problem(&ws_dir, &analysis, Some(path)) {
            Ok(_) => {
                state.streamdeck.set_active_problem(&analysis.problem_id);
                // Open VS Code
                open_in_vscode_internal(&analysis.problem_id, &ws_dir, state.settings.read().unwrap().open_pdf_in_vscode);
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
            state.streamdeck.set_active_problem(&pid);
            open_in_vscode_internal(&pid, &ws_dir, false);
            Json(serde_json::json!({ "success": true, "problem_id": pid }))
        }
        Err(e) => Json(serde_json::json!({ "success": false, "detail": e })),
    }
}

async fn get_staged_state() -> Json<serde_json::Value> {
    let stage_dir = AppSettings::staging_dir();
    let mut pdf_path: Option<String> = None;
    let mut zip_path: Option<String> = None;
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
                    }
                }
            }
        }
    }

    Json(serde_json::json!({
        "has_staged_pdf": pdf_path.is_some(),
        "has_staged_zip": zip_path.is_some(),
        "pdf_path": pdf_path,
        "zip_path": zip_path,
        "problem_id": problem_id,
        "title": title,
        "time_limit_sec": time_limit_sec.unwrap_or(1.0),
        "memory_limit_mb": memory_limit_mb.unwrap_or(128),
        "tests_count": tests_count,
    }))
}

async fn stage_file(Json(req): Json<StageFileRequest>) -> Json<serde_json::Value> {
    let src = Path::new(&req.file_path);
    if !src.exists() {
        return Json(serde_json::json!({ "success": false, "error": "Plik źródłowy nie istnieje" }));
    }

    let stage_dir = AppSettings::staging_dir();
    let _ = fs::create_dir_all(&stage_dir);

    let fname = src.file_name().unwrap_or_default();
    let dest = stage_dir.join(fname);
    let _ = fs::copy(src, &dest);

    Json(serde_json::json!({ "success": true, "path": dest.to_string_lossy() }))
}

async fn upload_staging(mut multipart: Multipart) -> Result<Json<serde_json::Value>, StatusCode> {
    let stage_dir = AppSettings::staging_dir();
    let _ = fs::create_dir_all(&stage_dir);

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.file_name().unwrap_or("upload").to_string();
        let target = stage_dir.join(&name);
        if let Ok(bytes) = field.bytes().await {
            let _ = fs::write(target, bytes);
        }
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn clear_staged() -> Json<serde_json::Value> {
    let stage_dir = AppSettings::staging_dir();
    if stage_dir.exists() {
        if let Ok(entries) = fs::read_dir(&stage_dir) {
            for e in entries.flatten() {
                let _ = fs::remove_file(e.path());
            }
        }
    }
    Json(serde_json::json!({ "success": true }))
}

async fn start_staged(State(state): State<AppState>) -> Json<serde_json::Value> {
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

    let ws_dir = state.settings.read().unwrap().get_workspace_path();

    if let Some(pdf) = pdf_file {
        if let Ok(text) = PdfParser::extract_text_from_file(&pdf) {
            let analysis = PdfParser::parse_heuristics(&text);
            let _p_dir = match WorkspaceManager::create_problem(&ws_dir, &analysis, Some(&pdf)) {
                Ok(p) => p,
                Err(e) => return Json(serde_json::json!({ "success": false, "error": e })),
            };

            if let Some(zip) = zip_file {
                if let Ok(bytes) = fs::read(&zip) {
                    let _ = WorkspaceManager::add_tests_from_zip(&ws_dir, &analysis.problem_id, &bytes);
                }
            }

            state.streamdeck.set_active_problem(&analysis.problem_id);
            open_in_vscode_internal(&analysis.problem_id, &ws_dir, state.settings.read().unwrap().open_pdf_in_vscode);
            let _ = clear_staged().await;

            return Json(serde_json::json!({
                "success": true,
                "problem_id": analysis.problem_id,
                "title": analysis.title,
            }));
        }
    }

    Json(serde_json::json!({ "success": false, "error": "Nie udało się sparsować pliku" }))
}

async fn set_active(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Json<serde_json::Value> {
    let ws_dir = state.settings.read().unwrap().get_workspace_path();
    let problems = WorkspaceManager::list_problems(&ws_dir);
    let cur_idx = problems.iter().position(|p| p.problem_id == id).unwrap_or(0) + 1;
    state.streamdeck.set_active_problem_with_meta(&id, cur_idx, problems.len().max(1));
    let _ = state.ws_sender.send(format!("active:{}", id));
    Json(serde_json::json!({ "success": true, "problem_id": id }))
}

async fn open_vscode(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Json<serde_json::Value> {
    let ws_dir = state.settings.read().unwrap().get_workspace_path();
    let split_pdf = state.settings.read().unwrap().open_pdf_in_vscode;
    open_in_vscode_internal(&id, &ws_dir, split_pdf);
    Json(serde_json::json!({ "success": true }))
}

fn open_in_vscode_internal(problem_id: &str, workspace_dir: &Path, split_pdf: bool) {
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

    #[cfg(target_os = "windows")]
    {
        let mut args = vec!["--reuse-window".to_string(), workspace_dir.to_string_lossy().to_string()];
        if sol_file.exists() {
            args.push(sol_file.to_string_lossy().to_string());
        }
        if split_pdf {
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

        let sd_code = dirs::home_dir().map(|h| h.join(".local/bin/sd_algo_code.sh"));
        let mut launched = false;
        if let Some(ref script) = sd_code {
            if script.exists() {
                let res = std::process::Command::new(script)
                    .arg(&p_dir)
                    .arg(&sol_file)
                    .env("DISPLAY", &display)
                    .env("XAUTHORITY", &xauthority)
                    .spawn();
                if res.is_ok() {
                    launched = true;
                }
            }
        }

        if !launched {
            let code_bin = if std::path::Path::new("/usr/bin/code").exists() {
                "/usr/bin/code"
            } else {
                "code"
            };
            let mut cmd = std::process::Command::new(code_bin);
            cmd.arg("--reuse-window");
            cmd.arg(workspace_dir);
            if sol_file.exists() {
                cmd.arg(&sol_file);
            }
            if split_pdf {
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
    AxumPath(id): AxumPath<String>,
) -> Json<serde_json::Value> {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("pkill").arg("-f").arg(&id).output();
    }
    state.streamdeck.update_key_status(8, "DANGER", Some("KILLED"));
    Json(serde_json::json!({ "success": true }))
}

async fn launch_screen(State(state): State<AppState>) -> Json<serde_json::Value> {
    state.streamdeck.set_idle_screen();
    Json(serde_json::json!({ "success": true }))
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
            open_in_vscode_internal(&pid, &ws_dir, state.settings.read().unwrap().open_pdf_in_vscode);
            return Json(serde_json::json!({ "success": true, "action": action }));
        }

        match action.as_str() {
            "open_vscode" => {
                open_in_vscode_internal(&active_id, &ws_dir, true);
            }
            "run_all" => {
                let _ = state.runner.run_all_tests(&active_id);
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
                    open_in_vscode_internal(&first.problem_id, &ws_dir, true);
                } else {
                    let _ = open_gui().await;
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
