use crate::config::AppSettings;
use crate::workspace::WorkspaceManager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResult {
    pub success: bool,
    pub target: String,
    pub debug_mode: bool,
    pub duration_ms: f64,
    pub warnings: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub verdict: String, // "OK", "WA", "TLE", "RE", "CE"
    pub execution_time_ms: f64,
    pub actual_output: String,
    pub expected_output: String,
    pub diff: String,
    pub error_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunAllSummary {
    pub success: bool,
    pub all_passed: bool,
    pub passed_count: usize,
    pub total_tests: usize,
    pub total_time_ms: f64,
    pub max_time_ms: f64,
    pub results: Vec<TestResult>,
}

pub struct TestExecutor {
    settings: Arc<RwLock<AppSettings>>,
}

impl TestExecutor {
    pub fn new(settings: Arc<RwLock<AppSettings>>) -> Self {
        Self { settings }
    }

    #[allow(dead_code)]
    pub fn new_with_settings(settings: AppSettings) -> Self {
        Self {
            settings: Arc::new(RwLock::new(settings)),
        }
    }

    fn resolve_compiler(&self, configured: &str) -> String {
        if let Ok(status) = Command::new(configured).arg("--version").output() {
            if status.status.success() {
                return configured.to_string();
            }
        }

        let candidates = [
            "clang++",
            "g++",
            "/opt/homebrew/bin/g++-14",
            "/opt/homebrew/bin/g++-13",
            "/opt/homebrew/bin/g++",
            "/opt/homebrew/bin/clang++",
            "/usr/local/bin/g++",
            "/usr/local/bin/clang++",
            "/usr/bin/clang++",
            "/usr/bin/g++",
            "c++",
            #[cfg(target_os = "windows")]
            "C:\\msys64\\ucrt64\\bin\\g++.exe",
            #[cfg(target_os = "windows")]
            "C:\\msys64\\mingw64\\bin\\g++.exe",
        ];

        for cand in candidates {
            if let Ok(out) = Command::new(cand).arg("--version").output() {
                if out.status.success() {
                    return cand.to_string();
                }
            }
        }

        configured.to_string()
    }

    pub fn compile(&self, problem_id: &str, debug_mode: bool) -> CompileResult {
        let p_id = problem_id.to_lowercase();
        let (ws_path, cpp_std, rel_flags, dbg_flags, raw_compiler) = {
            let s = self.settings.read().unwrap();
            (
                s.get_workspace_path(),
                s.cpp_standard.clone(),
                s.cxx_release_flags.clone(),
                s.cxx_debug_flags.clone(),
                s.cxx_compiler.clone(),
            )
        };
        let p_dir = WorkspaceManager::get_problem_dir(&ws_path, &p_id);
        let src_file = p_dir.join(format!("{}.cpp", p_id));

        if !src_file.exists() {
            return CompileResult {
                success: false,
                target: String::new(),
                debug_mode,
                duration_ms: 0.0,
                warnings: String::new(),
                error: format!("Plik źródłowy {}.cpp nie istnieje.", p_id),
            };
        }

        let target_name = if debug_mode {
            format!("{}_debug", p_id)
        } else {
            p_id.clone()
        };
        #[cfg(target_os = "windows")]
        let target_file = format!("{}.exe", target_name);
        #[cfg(not(target_os = "windows"))]
        let target_file = target_name.clone();

        let target_path = p_dir.join(&target_file);

        let flags_str = if debug_mode {
            &dbg_flags
        } else {
            &rel_flags
        };
        let flags: Vec<&str> = flags_str.split_whitespace().collect();

        let compiler = self.resolve_compiler(&raw_compiler);
        let mut cmd = Command::new(&compiler);
        cmd.current_dir(&p_dir);

        #[cfg(not(target_os = "windows"))]
        {
            let cur_path = std::env::var("PATH").unwrap_or_default();
            if !cur_path.contains("/opt/homebrew/bin") {
                cmd.env("PATH", format!("/opt/homebrew/bin:/usr/local/bin:{}", cur_path));
            }
        }

        cmd.arg(format!("-std={}", cpp_std));
        for f in flags {
            cmd.arg(f);
        }
        cmd.arg(format!("{}.cpp", p_id));
        cmd.arg("-o");
        cmd.arg(&target_file);

        let start = Instant::now();
        let output = match cmd.output() {
            Ok(out) => out,
            Err(e) => {
                return CompileResult {
                    success: false,
                    target: String::new(),
                    debug_mode,
                    duration_ms: 0.0,
                    warnings: String::new(),
                    error: format!("Błąd uruchomienia kompilatora {}: {}", compiler, e),
                };
            }
        };

        let duration_ms = (start.elapsed().as_micros() as f64) / 1000.0;
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            CompileResult {
                success: true,
                target: target_path.to_string_lossy().to_string(),
                debug_mode,
                duration_ms,
                warnings: stderr,
                error: String::new(),
            }
        } else {
            CompileResult {
                success: false,
                target: target_path.to_string_lossy().to_string(),
                debug_mode,
                duration_ms,
                warnings: String::new(),
                error: stderr,
            }
        }
    }

    pub fn run_single_test(&self, problem_id: &str, test_id: &str, is_debug: bool) -> TestResult {
        let p_id = problem_id.to_lowercase();
        let ws_path = self.settings.read().unwrap().get_workspace_path();
        let p_dir = WorkspaceManager::get_problem_dir(&ws_path, &p_id);
        let src_file = p_dir.join(format!("{}.cpp", p_id));

        let manifest = WorkspaceManager::get_manifest(&ws_path, &p_id);
        let time_limit_sec = manifest.as_ref().map_or(1.0, |m| m.time_limit_sec);

        let target_name = if is_debug {
            format!("{}_debug", p_id)
        } else {
            p_id.clone()
        };
        #[cfg(target_os = "windows")]
        let binary_file = format!("{}.exe", target_name);
        #[cfg(not(target_os = "windows"))]
        let binary_file = target_name;

        let binary_path = p_dir.join(&binary_file);

        let should_compile = if !binary_path.exists() {
            true
        } else if let (Ok(src_meta), Ok(bin_meta)) = (src_file.metadata(), binary_path.metadata()) {
            if let (Ok(src_mtime), Ok(bin_mtime)) = (src_meta.modified(), bin_meta.modified()) {
                src_mtime > bin_mtime
            } else {
                false
            }
        } else {
            false
        };

        if should_compile {
            let comp = self.compile(&p_id, is_debug);
            if !comp.success {
                return TestResult {
                    test_id: test_id.to_string(),
                    verdict: "CE".to_string(),
                    execution_time_ms: 0.0,
                    actual_output: String::new(),
                    expected_output: String::new(),
                    diff: comp.error.clone(),
                    error_message: comp.error,
                };
            }
        }

        let tests_dir = p_dir.join(".algo").join("tests");
        let in_file = tests_dir.join(format!("{}.in", test_id));
        let out_file = tests_dir.join(format!("{}.out", test_id));

        let input_bytes = fs::read(&in_file).unwrap_or_default();
        let expected_output = fs::read_to_string(&out_file).unwrap_or_default();

        let mut child = match Command::new(&binary_path)
            .current_dir(&p_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                return TestResult {
                    test_id: test_id.to_string(),
                    verdict: "RE".to_string(),
                    execution_time_ms: 0.0,
                    actual_output: String::new(),
                    expected_output,
                    diff: String::new(),
                    error_message: format!("Błąd uruchomienia binarki: {}", e),
                };
            }
        };

        let input_bytes_clone = input_bytes.clone();
        if let Some(mut stdin) = child.stdin.take() {
            std::thread::spawn(move || {
                let _ = stdin.write_all(&input_bytes_clone);
            });
        }

        let start = Instant::now();
        let timeout = Duration::from_secs_f64(time_limit_sec);
        let hard_timeout = timeout + Duration::from_millis(500);

        let mut timed_out = false;
        loop {
            match child.try_wait() {
                Ok(Some(_status)) => break,
                Ok(None) => {
                    if start.elapsed() > hard_timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        timed_out = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
            }
        }

        let duration_ms = (start.elapsed().as_micros() as f64) / 1000.0;

        if timed_out || duration_ms > (time_limit_sec * 1000.0) {
            return TestResult {
                test_id: test_id.to_string(),
                verdict: "TLE".to_string(),
                execution_time_ms: duration_ms,
                actual_output: String::new(),
                expected_output,
                diff: format!("Przekroczono limit czasu ({:.1}s).", time_limit_sec),
                error_message: "Time Limit Exceeded".to_string(),
            };
        }

        let output = match child.wait_with_output() {
            Ok(o) => o,
            Err(e) => {
                return TestResult {
                    test_id: test_id.to_string(),
                    verdict: "RE".to_string(),
                    execution_time_ms: duration_ms,
                    actual_output: String::new(),
                    expected_output,
                    diff: String::new(),
                    error_message: e.to_string(),
                };
            }
        };

        let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return TestResult {
                test_id: test_id.to_string(),
                verdict: "RE".to_string(),
                execution_time_ms: duration_ms,
                actual_output: stdout_str,
                expected_output,
                diff: stderr_str.clone(),
                error_message: format!("Proces zakończony z błędem: {}", stderr_str),
            };
        }

        // Compare output
        let norm_actual = Self::normalize_output(&stdout_str);
        let norm_expected = Self::normalize_output(&expected_output);

        if expected_output.trim().is_empty() || norm_actual == norm_expected {
            TestResult {
                test_id: test_id.to_string(),
                verdict: "OK".to_string(),
                execution_time_ms: duration_ms,
                actual_output: stdout_str,
                expected_output,
                diff: String::new(),
                error_message: String::new(),
            }
        } else {
            let diff_text = format!("Oczekiwano:\n{}\n\nOtrzymano:\n{}", norm_expected, norm_actual);
            TestResult {
                test_id: test_id.to_string(),
                verdict: "WA".to_string(),
                execution_time_ms: duration_ms,
                actual_output: stdout_str,
                expected_output,
                diff: diff_text,
                error_message: "Wrong Answer".to_string(),
            }
        }
    }

    pub fn run_all_tests(&self, problem_id: &str) -> RunAllSummary {
        let p_id = problem_id.to_lowercase();
        let ws_path = self.settings.read().unwrap().get_workspace_path();
        let p_dir = WorkspaceManager::get_problem_dir(&ws_path, &p_id);

        let manifest = WorkspaceManager::get_manifest(&ws_path, &p_id);
        let tests_dir = p_dir.join(".algo").join("tests");

        // Ensure compiled
        let comp = self.compile(&p_id, false);
        if !comp.success {
            return RunAllSummary {
                success: false,
                all_passed: false,
                passed_count: 0,
                total_tests: 0,
                total_time_ms: 0.0,
                max_time_ms: 0.0,
                results: vec![TestResult {
                    test_id: "compile".to_string(),
                    verdict: "CE".to_string(),
                    execution_time_ms: 0.0,
                    actual_output: String::new(),
                    expected_output: String::new(),
                    diff: comp.error.clone(),
                    error_message: comp.error,
                }],
            };
        }

        // Collect test IDs
        let mut test_ids = Vec::new();
        if let Some(m) = manifest {
            for t in m.tests {
                test_ids.push(t.id);
            }
        } else if tests_dir.exists() {
            if let Ok(entries) = fs::read_dir(&tests_dir) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.extension().and_then(|s| s.to_str()) == Some("in") {
                        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                            test_ids.push(stem.to_string());
                        }
                    }
                }
            }
        }

        test_ids.sort();

        let mut results = Vec::new();
        let mut passed_count = 0;
        let mut total_time_ms = 0.0;
        let mut max_time_ms = 0.0;
        let mut all_passed = true;

        for tid in &test_ids {
            let res = self.run_single_test(&p_id, tid, false);
            total_time_ms += res.execution_time_ms;
            if res.execution_time_ms > max_time_ms {
                max_time_ms = res.execution_time_ms;
            }

            if res.verdict == "OK" {
                passed_count += 1;
            } else {
                all_passed = false;
            }
            results.push(res);
        }

        RunAllSummary {
            success: true,
            all_passed: all_passed && !test_ids.is_empty(),
            passed_count,
            total_tests: test_ids.len(),
            total_time_ms,
            max_time_ms,
            results,
        }
    }

    fn normalize_output(text: &str) -> String {
        text.lines()
            .map(|l| l.trim_end())
            .filter(|l| !l.is_empty())
            .collect::<Vec<&str>>()
            .join("\n")
    }
}
