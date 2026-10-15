use serde_json::Value;
use std::io::{self, Read, Write};
use std::process::Command;

pub fn run_native_host() {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut length_buf = [0u8; 4];
    while stdin.read_exact(&mut length_buf).is_ok() {
        let msg_len = u32::from_ne_bytes(length_buf) as usize;
        let mut msg_buf = vec![0u8; msg_len];

        if stdin.read_exact(&mut msg_buf).is_err() {
            break;
        }

        let resp = match serde_json::from_slice::<Value>(&msg_buf) {
            Ok(json_msg) => handle_native_message(&json_msg),
            Err(e) => serde_json::json!({ "success": false, "error": e.to_string() }),
        };

        let resp_bytes = serde_json::to_vec(&resp).unwrap_or_default();
        let resp_len = (resp_bytes.len() as u32).to_ne_bytes();

        let _ = stdout.write_all(&resp_len);
        let _ = stdout.write_all(&resp_bytes);
        let _ = stdout.flush();
    }
}

fn handle_native_message(msg: &Value) -> Value {
    let action = msg.get("action").and_then(|v| v.as_str()).unwrap_or("start");
    let self_exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("algodeck"));

    match action {
        "start" => {
            crate::server::start_detached_daemon();
            let is_online = ureq::get("http://127.0.0.1:8080/api/status")
                .timeout(std::time::Duration::from_millis(400))
                .call()
                .is_ok();
            serde_json::json!({ "success": is_online, "status": if is_online { "online" } else { "starting" } })
        }
        "stop" => {
            crate::server::stop_daemon();
            serde_json::json!({ "success": true, "status": "stopped" })
        }
        "gui" => {
            let _ = Command::new(&self_exe).spawn();
            serde_json::json!({ "success": true, "status": "gui_launched" })
        }
        "stage_file" => {
            let file_path = msg.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
            if !file_path.is_empty() {
                let raw = file_path.trim().trim_start_matches("file://");
                let decoded = {
                    let mut out = Vec::new();
                    let bytes = raw.as_bytes();
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
                };

                let mut src = std::path::PathBuf::from(raw);
                if !src.exists() {
                    let dec_path = std::path::PathBuf::from(&decoded);
                    if dec_path.exists() {
                        src = dec_path;
                    } else {
                        let fname_opt = src.file_name().or_else(|| std::path::Path::new(&decoded).file_name()).map(|f| f.to_os_string());
                        if let Some(fname) = fname_opt {
                            if let Some(dl) = dirs::download_dir() {
                                let candidate = dl.join(&fname);
                                if candidate.exists() {
                                    src = candidate;
                                }
                            }
                            if !src.exists() {
                                if let Some(home) = dirs::home_dir() {
                                    let candidate = home.join("Downloads").join(&fname);
                                    if candidate.exists() {
                                        src = candidate;
                                    }
                                }
                            }
                        }
                    }
                }

                if src.exists() {
                    let stage_dir = crate::config::AppSettings::staging_dir();
                    let _ = std::fs::create_dir_all(&stage_dir);

                    let is_pdf = src.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("pdf")).unwrap_or(false);
                    if is_pdf {
                        if let Ok(entries) = std::fs::read_dir(&stage_dir) {
                            for e in entries.flatten() {
                                let p = e.path();
                                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                                    if ext.eq_ignore_ascii_case("pdf") {
                                        let _ = std::fs::remove_file(p);
                                    }
                                }
                            }
                        }
                    }

                    let filename = src.file_name().unwrap_or_default();
                    let dst = stage_dir.join(filename);
                    if std::fs::copy(&src, &dst).is_ok() {
                        let _ = ureq::post("http://127.0.0.1:8080/api/stage-refresh").call();
                        return serde_json::json!({ "success": true, "path": dst.to_string_lossy() });
                    }
                }
            }
            serde_json::json!({ "success": false })
        }
        "open_vscode" => {
            let pid = msg.get("problem_id").and_then(|v| v.as_str()).unwrap_or("");
            if !pid.is_empty() {
                let _ = ureq::post(&format!("http://127.0.0.1:8080/api/open-vscode/{}", pid)).call();
            }
            serde_json::json!({ "success": true, "status": "vscode_launched" })
        }
        _ => {
            serde_json::json!({ "success": false, "error": "unknown action" })
        }
    }
}
