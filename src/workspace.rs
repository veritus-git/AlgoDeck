use crate::pdf::ProblemAnalysis;
use serde::{Deserialize, Serialize};
use std::fs::{self};
use std::io::Read;
use std::path::{Path, PathBuf};

const CPP_TEMPLATE: &str = r#"/*
 * Zadanie: __TITLE__ (__PROBLEM_ID__)
 * Limit czasu: __TIME_LIMIT__s | Pamięć: __MEMORY_LIMIT__MB
 */

#include <iostream>
using namespace std;

int main() {
    ios_base::sync_with_stdio(false);
    cin.tie(NULL);

}
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemManifest {
    pub problem_id: String,
    pub title: String,
    pub time_limit_sec: f64,
    pub memory_limit_mb: u64,
    pub tests: Vec<ManifestTest>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestTest {
    pub id: String,
    pub name: String,
    pub in_file: String,
    pub out_file: String,
    pub tag: String,
    #[serde(default)]
    pub is_edge_case: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemSummary {
    pub problem_id: String,
    pub title: String,
    pub tests_count: usize,
    pub time_limit_sec: f64,
    pub memory_limit_mb: u64,
}

pub struct WorkspaceManager;

impl WorkspaceManager {
    pub fn get_problem_dir(workspace_dir: &Path, problem_id: &str) -> PathBuf {
        workspace_dir.join(problem_id.to_lowercase())
    }

    pub fn create_problem(
        workspace_dir: &Path,
        analysis: &ProblemAnalysis,
        original_pdf: Option<&Path>,
    ) -> Result<PathBuf, String> {
        let p_id = analysis.problem_id.trim().to_lowercase();
        if p_id.is_empty() {
            return Err("Nieprawidłowy kod zadania".to_string());
        }

        let p_dir = Self::get_problem_dir(workspace_dir, &p_id);
        fs::create_dir_all(&p_dir).map_err(|e| e.to_string())?;

        let algo_dir = p_dir.join(".algo");
        fs::create_dir_all(&algo_dir).map_err(|e| e.to_string())?;

        let tests_dir = algo_dir.join("tests");
        fs::create_dir_all(&tests_dir).map_err(|e| e.to_string())?;

        // 1. Solution .cpp file
        let main_cpp = p_dir.join(format!("{}.cpp", p_id));
        if !main_cpp.exists() {
            let content = CPP_TEMPLATE
                .replace("__TITLE__", &analysis.title)
                .replace("__PROBLEM_ID__", &p_id)
                .replace("__TIME_LIMIT__", &analysis.time_limit_sec.to_string())
                .replace("__MEMORY_LIMIT__", &analysis.memory_limit_mb.to_string());
            fs::write(&main_cpp, content).map_err(|e| e.to_string())?;
        }

        // 2. Copy PDF statement if available
        if let Some(pdf) = original_pdf {
            if pdf.exists() {
                let target_pdf = algo_dir.join("statement.pdf");
                let _ = fs::copy(pdf, target_pdf);
            }
        }

        // 3. Save tests
        let mut manifest_tests: Vec<ManifestTest> = Vec::new();
        for (i, t) in analysis.tests.iter().enumerate() {
            let tid = if t.id.trim().is_empty() {
                format!("test_{}", i + 1)
            } else {
                t.id.trim().to_string()
            };

            let in_file_name = format!("{}.in", tid);
            let out_file_name = format!("{}.out", tid);
            let in_path = tests_dir.join(&in_file_name);
            let out_path = tests_dir.join(&out_file_name);

            let in_body = if t.input.trim().is_empty() { "\n".to_string() } else { format!("{}\n", t.input.trim()) };
            let _ = fs::write(&in_path, in_body);

            if !t.expected_output.trim().is_empty() {
                let _ = fs::write(&out_path, format!("{}\n", t.expected_output.trim()));
            }

            manifest_tests.push(ManifestTest {
                id: tid,
                name: t.name.clone(),
                in_file: in_file_name,
                out_file: if t.expected_output.trim().is_empty() { String::new() } else { out_file_name },
                tag: t.tag.clone(),
                is_edge_case: t.is_edge_case,
            });
        }

        // 4. Save manifest
        let manifest = ProblemManifest {
            problem_id: p_id.clone(),
            title: analysis.title.clone(),
            time_limit_sec: analysis.time_limit_sec,
            memory_limit_mb: analysis.memory_limit_mb,
            tests: manifest_tests,
            created_at: chrono::Local::now().to_rfc3339(),
        };

        let manifest_path = algo_dir.join("problem.json");
        let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
        fs::write(manifest_path, manifest_json).map_err(|e| e.to_string())?;

        Ok(p_dir)
    }

    pub fn get_manifest(workspace_dir: &Path, problem_id: &str) -> Option<ProblemManifest> {
        let p_dir = Self::get_problem_dir(workspace_dir, problem_id);
        let path = p_dir.join(".algo").join("problem.json");
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                return serde_json::from_str::<ProblemManifest>(&content).ok();
            }
        }
        None
    }

    pub fn list_problems(workspace_dir: &Path) -> Vec<ProblemSummary> {
        let mut list = Vec::new();
        if !workspace_dir.exists() || !workspace_dir.is_dir() {
            return list;
        }

        if let Ok(entries) = fs::read_dir(workspace_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if name.starts_with('.') || name.eq_ignore_ascii_case("tests") {
                        continue;
                    }

                    let p_id = name.to_lowercase();
                    let mut title = p_id.to_uppercase();
                    let mut time_limit = 1.0;
                    let mut memory_limit = 128;
                    let mut tests_count = 0;

                    if let Some(m) = Self::get_manifest(workspace_dir, &p_id) {
                        title = m.title;
                        time_limit = m.time_limit_sec;
                        memory_limit = m.memory_limit_mb;
                        tests_count = m.tests.len();
                    } else {
                        // Fallback counting .in files in .algo/tests
                        let tests_dir = p.join(".algo").join("tests");
                        if tests_dir.exists() {
                            if let Ok(t_entries) = fs::read_dir(tests_dir) {
                                for te in t_entries.flatten() {
                                    if te.path().extension().and_then(|s| s.to_str()) == Some("in") {
                                        tests_count += 1;
                                    }
                                }
                            }
                        }
                    }

                    list.push(ProblemSummary {
                        problem_id: p_id,
                        title,
                        tests_count,
                        time_limit_sec: time_limit,
                        memory_limit_mb: memory_limit,
                    });
                }
            }
        }

        list.sort_by(|a, b| a.problem_id.cmp(&b.problem_id));
        list
    }

    pub fn delete_problem(workspace_dir: &Path, problem_id: &str) -> Result<(), String> {
        let p_dir = Self::get_problem_dir(workspace_dir, problem_id);
        if p_dir.exists() {
            fs::remove_dir_all(&p_dir).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn add_tests_from_zip(
        workspace_dir: &Path,
        problem_id: &str,
        zip_bytes: &[u8],
    ) -> Result<usize, String> {
        let p_dir = Self::get_problem_dir(workspace_dir, problem_id);
        if !p_dir.exists() {
            return Err("Katalog zadania nie istnieje".to_string());
        }

        let tests_dir = p_dir.join(".algo").join("tests");
        fs::create_dir_all(&tests_dir).map_err(|e| e.to_string())?;

        let reader = std::io::Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("Błąd rozpakowywania ZIP: {}", e))?;

        let mut added = 0;
        let mut extracted_files: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            if file.is_file() {
                let fname = file.name().to_string();
                let clean_name = Path::new(&fname)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                if !clean_name.starts_with('.') {
                    let mut buffer = Vec::new();
                    let _ = file.read_to_end(&mut buffer);
                    extracted_files.insert(clean_name, buffer);
                }
            }
        }

        // Pair up .in with .out or .ans
        let mut in_stems: Vec<String> = Vec::new();
        for name in extracted_files.keys() {
            let p = Path::new(name);
            if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("in") {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        in_stems.push(stem.to_string());
                    }
                }
            }
        }

        for stem in in_stems {
            let in_name = format!("{}.in", stem);
            if let Some(in_bytes) = extracted_files.get(&in_name) {
                let target_in = tests_dir.join(format!("{}.in", stem));
                let _ = fs::write(&target_in, in_bytes);

                // Look for out or ans
                let out_name = format!("{}.out", stem);
                let ans_name = format!("{}.ans", stem);

                if let Some(out_bytes) = extracted_files.get(&out_name) {
                    let target_out = tests_dir.join(format!("{}.out", stem));
                    let _ = fs::write(&target_out, out_bytes);
                } else if let Some(ans_bytes) = extracted_files.get(&ans_name) {
                    let target_out = tests_dir.join(format!("{}.out", stem));
                    let _ = fs::write(&target_out, ans_bytes);
                }

                added += 1;
            }
        }

        // Update manifest with new test list
        if let Some(mut m) = Self::get_manifest(workspace_dir, problem_id) {
            let mut manifest_tests = Vec::new();
            if let Ok(td_entries) = fs::read_dir(&tests_dir) {
                for te in td_entries.flatten() {
                    let path = te.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("in") {
                        let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                        let out_f = tests_dir.join(format!("{}.out", stem));
                        manifest_tests.push(ManifestTest {
                            id: stem.clone(),
                            name: format!("Test {}", stem),
                            in_file: format!("{}.in", stem),
                            out_file: if out_f.exists() { format!("{}.out", stem) } else { String::new() },
                            tag: "[PAKIET TESTÓW]".to_string(),
                            is_edge_case: false,
                        });
                    }
                }
            }
            manifest_tests.sort_by(|a, b| a.id.cmp(&b.id));
            m.tests = manifest_tests;

            let m_path = p_dir.join(".algo").join("problem.json");
            if let Ok(m_json) = serde_json::to_string_pretty(&m) {
                let _ = fs::write(m_path, m_json);
            }
        }

        Ok(added)
    }
}
