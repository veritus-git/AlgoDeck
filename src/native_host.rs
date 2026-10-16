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

pub fn ensure_native_manifests() {
    let allowed_origins = vec![
        "chrome-extension://jglgkkeklbipodmhcncnmdnippdijjcp/",
        "chrome-extension://cdgdnkfkkbadljdgjjjbaefefcbelplf/",
        "chrome-extension://jnfimdneicefaomnlikmogffogcadabf/",
    ];

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            let bin_dir = home.join(".local/bin");
            let _ = std::fs::create_dir_all(&bin_dir);
            let wrapper_path = bin_dir.join("algodeck-native-host");

            let current_exe = std::env::current_exe().unwrap_or_else(|_| bin_dir.join("algodeck"));
            let wrapper_script = format!(
                "#!/bin/sh\nexec \"{}\" native-host\n",
                current_exe.to_string_lossy()
            );
            let _ = std::fs::write(&wrapper_path, wrapper_script);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&wrapper_path, std::fs::Permissions::from_mode(0o755));
            }

            let manifest = serde_json::json!({
                "name": "com.algodeck.launcher",
                "description": "AlgoDeck 100% Rust Native Messaging Host",
                "path": wrapper_path.to_string_lossy(),
                "type": "stdio",
                "allowed_origins": allowed_origins
            });
            let manifest_str = serde_json::to_string_pretty(&manifest).unwrap_or_default();

            let target_dirs = [
                home.join(".config/google-chrome/NativeMessagingHosts"),
                home.join(".config/chromium/NativeMessagingHosts"),
                home.join(".config/BraveSoftware/Brave-Browser/NativeMessagingHosts"),
                home.join(".config/microsoft-edge/NativeMessagingHosts"),
                home.join(".config/vivaldi/NativeMessagingHosts"),
                home.join(".config/opera/NativeMessagingHosts"),
            ];

            for dir in &target_dirs {
                let _ = std::fs::create_dir_all(dir);
                let manifest_file = dir.join("com.algodeck.launcher.json");
                let _ = std::fs::write(manifest_file, &manifest_str);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let bin_dir = home.join(".local/bin");
            let _ = std::fs::create_dir_all(&bin_dir);
            let wrapper_path = bin_dir.join("algodeck-native-host");

            let current_exe = std::env::current_exe().unwrap_or_else(|_| bin_dir.join("algodeck"));
            let wrapper_script = format!(
                "#!/bin/sh\nexec \"{}\" native-host\n",
                current_exe.to_string_lossy()
            );
            let _ = std::fs::write(&wrapper_path, wrapper_script);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&wrapper_path, std::fs::Permissions::from_mode(0o755));
            }

            let manifest = serde_json::json!({
                "name": "com.algodeck.launcher",
                "description": "AlgoDeck 100% Rust Native Messaging Host for macOS",
                "path": wrapper_path.to_string_lossy(),
                "type": "stdio",
                "allowed_origins": allowed_origins
            });
            let manifest_str = serde_json::to_string_pretty(&manifest).unwrap_or_default();

            let target_dirs = [
                home.join("Library/Application Support/Google/Chrome/NativeMessagingHosts"),
                home.join("Library/Application Support/BraveSoftware/Brave-Browser/NativeMessagingHosts"),
                home.join("Library/Application Support/Chromium/NativeMessagingHosts"),
                home.join("Library/Application Support/Microsoft Edge/NativeMessagingHosts"),
                home.join("Library/Application Support/Vivaldi/NativeMessagingHosts"),
            ];

            for dir in &target_dirs {
                let _ = std::fs::create_dir_all(dir);
                let manifest_file = dir.join("com.algodeck.launcher.json");
                let _ = std::fs::write(manifest_file, &manifest_str);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let app_dir = std::path::PathBuf::from(&local).join("AlgoDeck");
            let _ = std::fs::create_dir_all(&app_dir);
            let exe_path = std::env::current_exe().unwrap_or_else(|_| app_dir.join("algodeck.exe"));
            let manifest = serde_json::json!({
                "name": "com.algodeck.launcher",
                "description": "AlgoDeck 100% Rust Native Messaging Host for Windows",
                "path": exe_path.to_string_lossy(),
                "type": "stdio",
                "allowed_origins": allowed_origins
            });
            let manifest_file = app_dir.join("com.algodeck.launcher.json");
            if let Ok(manifest_str) = serde_json::to_string_pretty(&manifest) {
                let _ = std::fs::write(&manifest_file, &manifest_str);
            }
        }
    }
}

