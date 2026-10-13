use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const API_BASE: &str = "http://127.0.0.1:8080";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemInfo {
    pub problem_id: String,
    pub title: String,
    #[serde(default)]
    pub tests_count: usize,
    #[serde(default)]
    pub time_limit_sec: f64,
    #[serde(default)]
    pub memory_limit_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoImportResponse {
    pub success: Option<bool>,
    pub problem_id: Option<String>,
    pub title: Option<String>,
    pub time_limit_sec: Option<f64>,
    pub memory_limit_mb: Option<u64>,
    pub tests_count: Option<usize>,
    pub detail: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub version: String,
    pub active_problem: Option<String>,
    pub workspace_dir: Option<String>,
}

pub struct ApiClient;

impl ApiClient {
    pub fn is_alive() -> bool {
        match ureq::get(&format!("{}/api/status", API_BASE))
            .timeout(std::time::Duration::from_millis(600))
            .call()
        {
            Ok(res) => res.status() == 200,
            Err(_) => false,
        }
    }

    #[allow(dead_code)]
    pub fn get_status() -> Option<StatusResponse> {
        let res = ureq::get(&format!("{}/api/status", API_BASE))
            .timeout(std::time::Duration::from_millis(800))
            .call()
            .ok()?;
        res.into_json().ok()
    }

    pub fn auto_parse_pdf(file_path: &str) -> Option<AutoImportResponse> {
        let payload = serde_json::json!({
            "file_path": file_path,
            "auto_create": false
        });
        let res = ureq::post(&format!("{}/api/auto-import-pdf", API_BASE))
            .timeout(std::time::Duration::from_secs(3))
            .send_json(payload)
            .ok()?;
        res.into_json().ok()
    }

    pub fn create_task_pdf(
        file_path: &str,
        problem_id: &str,
        title: &str,
        time_limit: f64,
        memory_limit: u64,
    ) -> Result<AutoImportResponse, String> {
        let payload = serde_json::json!({
            "file_path": file_path,
            "problem_id": problem_id,
            "title": title,
            "time_limit": time_limit,
            "memory_limit": memory_limit,
            "auto_create": true
        });
        let res = ureq::post(&format!("{}/api/auto-import-pdf", API_BASE))
            .timeout(std::time::Duration::from_secs(10))
            .send_json(payload)
            .map_err(|e| e.to_string())?;

        res.into_json::<AutoImportResponse>()
            .map_err(|e| e.to_string())
    }

    pub fn create_task_manual(
        problem_id: &str,
        title: &str,
        time_limit: f64,
        memory_limit: u64,
        cin: &str,
        cout: &str,
    ) -> Result<(), String> {
        let mut tests = Vec::new();
        if !cin.trim().is_empty() || !cout.trim().is_empty() {
            tests.push(serde_json::json!({
                "id": "test_1",
                "name": "Przykład 1",
                "input": cin,
                "expected_output": cout
            }));
        }

        let payload = serde_json::json!({
            "problem_id": problem_id,
            "title": title,
            "time_limit": time_limit,
            "memory_limit": memory_limit,
            "tests": tests
        });

        let res = ureq::post(&format!("{}/api/create-manual", API_BASE))
            .timeout(std::time::Duration::from_secs(6))
            .send_json(payload)
            .map_err(|e| e.to_string())?;

        if res.status() == 200 {
            Ok(())
        } else {
            Err(format!("Błąd serwera: kod {}", res.status()))
        }
    }

    pub fn fetch_problems(workspace_dir: &str) -> (Vec<ProblemInfo>, String) {
        // Spróbuj przez API
        if let Ok(res) = ureq::get(&format!("{}/api/problems", API_BASE))
            .timeout(std::time::Duration::from_millis(1500))
            .call()
        {
            if let Ok(json) = res.into_json::<serde_json::Value>() {
                let active = json
                    .get("active")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let mut list = Vec::new();
                if let Some(arr) = json.get("problems").and_then(|v| v.as_array()) {
                    for item in arr {
                        let pid = item.get("problem_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let title = item.get("title").and_then(|v| v.as_str()).unwrap_or(&pid).to_string();
                        let tests_count = item
                            .get("tests")
                            .and_then(|v| v.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);
                        let time_limit = item.get("time_limit").and_then(|v| v.as_f64()).unwrap_or(1.0);
                        let memory_limit = item.get("memory_limit").and_then(|v| v.as_u64()).unwrap_or(128);

                        list.push(ProblemInfo {
                            problem_id: pid,
                            title,
                            tests_count,
                            time_limit_sec: time_limit,
                            memory_limit_mb: memory_limit,
                        });
                    }
                }
                return (list, active);
            }
        }

        // Fallback do skanowania dysku
        let mut list = Vec::new();
        let path = Path::new(workspace_dir);
        if path.exists() && path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let ep = entry.path();
                    if ep.is_dir() {
                        let name = ep.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if name.starts_with('.') || name.eq_ignore_ascii_case("tests") {
                            continue;
                        }

                        let pid = name.to_lowercase();
                        let mut title = pid.to_uppercase();
                        let mut t_count = 0;
                        let mut time_lim = 1.0;
                        let mut mem_lim = 128;

                        let meta_path = ep.join(".algo").join("problem.json");
                        if meta_path.exists() {
                            if let Ok(content) = fs::read_to_string(&meta_path) {
                                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                                    if let Some(t) = parsed.get("title").and_then(|v| v.as_str()) {
                                        title = t.to_string();
                                    }
                                    if let Some(tl) = parsed.get("time_limit").and_then(|v| v.as_f64()) {
                                        time_lim = tl;
                                    }
                                    if let Some(ml) = parsed.get("memory_limit").and_then(|v| v.as_u64()) {
                                        mem_lim = ml;
                                    }
                                }
                            }
                        }

                        let tests_dir = ep.join(".algo").join("tests");
                        if tests_dir.exists() {
                            if let Ok(td_entries) = fs::read_dir(&tests_dir) {
                                for t_entry in td_entries.flatten() {
                                    if t_entry.path().extension().and_then(|e| e.to_str()) == Some("in") {
                                        t_count += 1;
                                    }
                                }
                            }
                        }

                        list.push(ProblemInfo {
                            problem_id: pid,
                            title,
                            tests_count: t_count,
                            time_limit_sec: time_lim,
                            memory_limit_mb: mem_lim,
                        });
                    }
                }
            }
        }

        list.sort_by(|a, b| a.problem_id.cmp(&b.problem_id));
        (list, String::new())
    }

    pub fn set_active_problem(problem_id: &str) -> Result<(), String> {
        let url = format!("{}/api/set-active/{}", API_BASE, problem_id);
        let _ = ureq::post(&url).timeout(std::time::Duration::from_secs(2)).call();

        // Fallback / natywny skrypt przełączania
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("bash")
                .arg("-c")
                .arg(format!("$HOME/.local/bin/sd_algo_switch.sh to '{}'", problem_id))
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("bash")
                .arg("-c")
                .arg(format!("$HOME/.local/bin/sd_algo_switch.sh to '{}'", problem_id))
                .spawn();
        }

        Ok(())
    }

    pub fn delete_problem(problem_id: &str, workspace_dir: &str) -> Result<(), String> {
        let url = format!("{}/api/problem/{}", API_BASE, problem_id);
        let _ = ureq::delete(&url).timeout(std::time::Duration::from_secs(3)).call();

        let path = PathBuf::from(workspace_dir).join(problem_id);
        if path.exists() {
            let _ = fs::remove_dir_all(&path);
        }
        Ok(())
    }

    pub fn show_idle_screen() {
        let _ = ureq::post(&format!("{}/api/streamdeck/page/ALGO_IDLE", API_BASE))
            .timeout(std::time::Duration::from_millis(800))
            .call();

        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("gdbus")
                .args([
                    "call",
                    "--session",
                    "--dest",
                    "com.core447.StreamController",
                    "--object-path",
                    "/com/core447/StreamController",
                    "--method",
                    "com.core447.StreamController.ChangePage",
                    "A00SA6042JGA63",
                    "ALGO_IDLE",
                ])
                .spawn();
        }
    }

    pub fn sync_streamdeck() {
        let _ = ureq::post(&format!("{}/api/streamdeck/sync", API_BASE))
            .timeout(std::time::Duration::from_secs(5))
            .call();
    }

    pub fn open_in_vscode(problem_id: &str, workspace_dir: &str) {
        let _ = ureq::post(&format!("{}/api/open-vscode/{}", API_BASE, problem_id))
            .timeout(std::time::Duration::from_millis(800))
            .call();

        let target_dir = PathBuf::from(workspace_dir).join(problem_id);
        let target_file = target_dir.join(format!("{}.cpp", problem_id));

        if let Some(home) = dirs::home_dir() {
            let script = home.join(".local/bin/sd_algo_code.sh");
            if script.exists() {
                let _ = std::process::Command::new("bash")
                    .arg(&script)
                    .arg(&target_dir)
                    .arg(&target_file)
                    .spawn();
                return;
            }
        }

        let _ = std::process::Command::new("code")
            .arg("--reuse-window")
            .arg(workspace_dir)
            .arg(&target_file)
            .spawn();
    }

    pub fn run_code(problem_id: &str, workspace_dir: &str) {
        let pdir = PathBuf::from(workspace_dir).join(problem_id);
        if let Some(home) = dirs::home_dir() {
            let script = home.join(".local/bin/sd_algo_run_vscode.sh");
            if script.exists() {
                let _ = std::process::Command::new("bash")
                    .arg(&script)
                    .arg(&pdir)
                    .arg(problem_id)
                    .spawn();
                return;
            }
        }
        let _ = ureq::post(&format!("{}/api/run/{}", API_BASE, problem_id))
            .timeout(std::time::Duration::from_millis(800))
            .call();
    }

    pub fn test_code(problem_id: &str, workspace_dir: &str) {
        let pdir = PathBuf::from(workspace_dir).join(problem_id);
        if let Some(home) = dirs::home_dir() {
            let script = home.join(".local/bin/sd_algo_test_vscode.sh");
            if script.exists() {
                let _ = std::process::Command::new("bash")
                    .arg(&script)
                    .arg(&pdir)
                    .arg(problem_id)
                    .spawn();
                return;
            }
        }
        let _ = ureq::post(&format!("{}/api/test/{}", API_BASE, problem_id))
            .timeout(std::time::Duration::from_millis(800))
            .call();
    }

    pub fn add_tests_zip(problem_id: &str, zip_path: &Path) -> Result<usize, String> {
        let file_bytes = fs::read(zip_path).map_err(|e| e.to_string())?;
        let filename = zip_path.file_name().unwrap_or_default().to_string_lossy().to_string();

        let boundary = "------------------------algodeckboundary123";
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(format!("Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n", filename).as_bytes());
        body.extend_from_slice(b"Content-Type: application/zip\r\n\r\n");
        body.extend_from_slice(&file_bytes);
        body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

        let res = ureq::post(&format!("{}/api/problems/{}/add-tests", API_BASE, problem_id))
            .set("Content-Type", &format!("multipart/form-data; boundary={}", boundary))
            .timeout(std::time::Duration::from_secs(30))
            .send_bytes(&body)
            .map_err(|e| e.to_string())?;

        let json: serde_json::Value = res.into_json().map_err(|e| e.to_string())?;
        Ok(json.get("tests_added").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
    }

    pub fn exit_algodeck() {
        let _ = ureq::post(&format!("{}/api/exit", API_BASE))
            .timeout(std::time::Duration::from_millis(800))
            .call();
    }

    pub fn shutdown() {
        let _ = ureq::post(&format!("{}/api/shutdown", API_BASE))
            .timeout(std::time::Duration::from_millis(800))
            .call();
        let _ = std::process::Command::new("bash")
            .arg("-c")
            .arg("$HOME/.local/bin/algodeck stop || true")
            .spawn();
    }

    pub fn start_daemon() {
        let _ = std::process::Command::new("bash")
            .arg("-c")
            .arg("$HOME/.local/bin/algodeck start || true")
            .spawn();
    }

    pub fn update_backend_settings(settings: &crate::settings::AppSettings) {
        let payload = serde_json::json!({
            "workspace_dir": settings.workspace_dir,
            "vscode_mode": settings.vscode_mode,
            "notifications": settings.notifications,
            "auto_detect_downloads": settings.auto_detect_downloads,
        });
        let _ = ureq::post(&format!("{}/api/settings", API_BASE))
            .timeout(std::time::Duration::from_millis(800))
            .send_json(payload);
    }
}
