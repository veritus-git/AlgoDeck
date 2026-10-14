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
            let _ = Command::new(&self_exe).arg("start").spawn();
            serde_json::json!({ "success": true, "status": "starting" })
        }
        "stop" => {
            let _ = Command::new(&self_exe).arg("stop").spawn();
            serde_json::json!({ "success": true, "status": "stopping" })
        }
        "gui" => {
            let _ = Command::new(&self_exe).spawn();
            serde_json::json!({ "success": true, "status": "gui_launched" })
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
