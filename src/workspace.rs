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
            let target_pdf = algo_dir.join("statement.pdf");
            if pdf.exists() && pdf != target_pdf {
                if let Ok(meta) = fs::metadata(pdf) {
                    if meta.len() > 0 {
                        let _ = fs::copy(pdf, target_pdf);
                    }
                }
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

        // 5. Generate run.sh, test.sh, kill.sh, accept.sh & .vscode/tasks.json
        Self::ensure_problem_scripts(workspace_dir, &p_id, &analysis.title, analysis.time_limit_sec);

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
        if !workspace_dir.exists() || !workspace_dir.is_dir() {
            return Vec::new();
        }

        let mut list: Vec<(ProblemSummary, i64)> = Vec::new();

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

                    let mut created_ts: i64 = 0;
                    if let Some(m) = Self::get_manifest(workspace_dir, &p_id) {
                        title = m.title;
                        time_limit = m.time_limit_sec;
                        memory_limit = m.memory_limit_mb;
                        tests_count = m.tests.len();
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&m.created_at) {
                            created_ts = dt.timestamp_millis();
                        }
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

                    if created_ts == 0 {
                        if let Ok(meta) = entry.metadata() {
                            let sys_time = meta.created().or_else(|_| meta.modified()).unwrap_or(std::time::UNIX_EPOCH);
                            created_ts = sys_time
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as i64;
                        }
                    }

                    let test_sh = p.join(".algo").join("test.sh");
                    if !test_sh.exists() {
                        Self::ensure_problem_scripts(workspace_dir, &p_id, &title, time_limit);
                    }

                    list.push((
                        ProblemSummary {
                            problem_id: p_id,
                            title,
                            tests_count,
                            time_limit_sec: time_limit,
                            memory_limit_mb: memory_limit,
                        },
                        created_ts,
                    ));
                }
            }
        }

        // Sortowanie chronologiczne według kolejności dodania zadań
        list.sort_by(|(a_summary, a_ts), (b_summary, b_ts)| {
            a_ts.cmp(b_ts).then_with(|| a_summary.problem_id.cmp(&b_summary.problem_id))
        });

        list.into_iter().map(|(summary, _)| summary).collect()
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

    pub fn ensure_problem_scripts(
        workspace_dir: &Path,
        problem_id: &str,
        title: &str,
        time_limit_sec: f64,
    ) {
        let p_id = problem_id.trim().to_lowercase();
        if p_id.is_empty() {
            return;
        }
        let p_dir = Self::get_problem_dir(workspace_dir, &p_id);
        let algo_dir = p_dir.join(".algo");
        let _ = fs::create_dir_all(&algo_dir);

        let time_limit_ms = (time_limit_sec * 1000.0).round() as u64;

        // 1. run.sh
        let run_sh_path = algo_dir.join("run.sh");
        let run_content = format!(
            r#"#!/usr/bin/env bash
cd "{p_dir_disp}"

echo -e "\e[1;36m[AlgoDeck] Kompilacja {p_id}.cpp (g++ -O3 -std=c++20)...\e[0m"
mkdir -p .algo
if ! g++ -O3 -std=c++20 "{p_id}.cpp" -o ".algo/{p_id}" 2>.algo/compile.log; then
    echo -e "\e[1;31m❌ BŁĄD KOMPILACJI!\e[0m"
    echo -e "\e[1;31m"
    cat .algo/compile.log
    echo -e "\e[0m"
    exit 1
fi

echo -e "\e[1;32m✓ Skompilowano pomyślnie!\e[0m"
echo -e "\e[1;33m▶ Program uruchomiony (wprowadź dane cin):\e[0m"
echo -e "\e[0;90m---------------------------------------------------------\e[0m"

./.algo/{p_id}
RET=$?

echo ""
echo -e "\e[0;90m---------------------------------------------------------\e[0m"
if [ "${{RET:-0}}" -eq 0 ]; then
    echo -e "\e[1;32m✓ Program zakończony pomyślnie (kod: 0).\e[0m"
else
    echo -e "\e[1;31m❌ Program zakończony błędem (kod: ${{RET}}).\e[0m"
fi
"#,
            p_dir_disp = p_dir.display(),
            p_id = p_id
        );
        let _ = fs::write(&run_sh_path, run_content);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&run_sh_path, fs::Permissions::from_mode(0o755));
        }

        // 2. test.sh
        let test_sh_path = algo_dir.join("test.sh");
        let test_content = format!(
            r#"#!/usr/bin/env bash
set -o pipefail
cd "{p_dir_disp}"

VERBOSE=false
WAIT_AT_END=false
for arg in "$@"; do
    case "$arg" in
        -v|--verbose|-d|--details) VERBOSE=true ;;
        -w|--wait) WAIT_AT_END=true ;;
    esac
done

PROB="{p_id}"
PROB_UPPER="{p_upper}"
PROB_TITLE="{title}"
TIME_LIMIT_MS={time_limit_ms}

C_RESET="\e[0m"
C_BOLD="\e[1m"
C_DIM="\e[2m"
C_GREEN="\e[1;32m"
C_RED="\e[1;31m"
C_YELLOW="\e[1;33m"
C_BLUE="\e[1;34m"
C_CYAN="\e[1;36m"
C_MAGENTA="\e[1;35m"
C_WHITE="\e[1;37m"
C_GRAY="\e[0;90m"

pad_left() {{
    local text="$1"
    local target_width="$2"
    local len=${{#text}}
    local pad_len=$(( target_width - len ))
    if [ $pad_len -gt 0 ]; then
        printf "%*s%s" "$pad_len" "" "$text"
    else
        printf "%s" "$text"
    fi
}}

pad_right() {{
    local text="$1"
    local target_width="$2"
    local len=${{#text}}
    local extra=0
    [[ "$text" =~ "✨" ]] && extra=$((extra + 1))
    [[ "$text" =~ "❌" ]] && extra=$((extra + 1))
    [[ "$text" =~ "⚠️" ]] && extra=$((extra + 1))
    local visual_len=$(( len + extra ))
    local pad_len=$(( target_width - visual_len ))
    if [ $pad_len -gt 0 ]; then
        printf "%s%*s" "$text" "$pad_len" ""
    else
        printf "%s" "$text"
    fi
}}

box_line() {{
    local text="$1"
    local total_inner_width=78
    local char_len=${{#text}}
    local extra=0
    [[ "$text" =~ "✨" ]] && extra=$((extra + 1))
    [[ "$text" =~ "❌" ]] && extra=$((extra + 1))
    [[ "$text" =~ "⚠️" ]] && extra=$((extra + 1))
    local visual_len=$(( char_len + extra ))
    local pad=$(( total_inner_width - visual_len ))
    [ $pad -lt 0 ] && pad=0
    printf "║%s%*s║\n" "$text" "$pad" ""
}}

DIVIDER="$(printf "=%.0s" {{1..80}})"
echo -e "${{C_CYAN}}${{C_BOLD}}${{DIVIDER}}${{C_RESET}}"
echo -e "${{C_CYAN}}${{C_BOLD}}       ⚡ AlgoDeck Test Runner — ${{PROB_UPPER}} (${{PROB_TITLE}})${{C_RESET}}"
echo -e "${{C_CYAN}}${{C_BOLD}}${{DIVIDER}}${{C_RESET}}"

echo -ne "${{C_YELLOW}}⚡ Kompilacja ${{PROB}}.cpp (g++ -O3 -std=c++20)...${{C_RESET}} "
mkdir -p .algo
if ! g++ -O3 -std=c++20 "${{PROB}}.cpp" -o ".algo/${{PROB}}" 2>.algo/compile.log; then
    echo -e "${{C_RED}}${{C_BOLD}}❌ BŁĄD KOMPILACJI!${{C_RESET}}"
    echo -e "${{C_RED}}"
    cat .algo/compile.log
    echo -e "${{C_RESET}}"
    notify-send -u critical "AlgoDeck (${{PROB_UPPER}})" "❌ Błąd kompilacji!" 2>/dev/null || true
    [ "$WAIT_AT_END" = true ] && read -p "Naciśnij Enter, aby zamknąć..."
    exit 1
fi
echo -e "${{C_GREEN}}${{C_BOLD}}✓ OK${{C_RESET}}"
echo ""

TOTAL=0
PASSED=0
FAILED=0
ERRORS=()

TEST_FILES=($(ls -1 .algo/tests/*.in 2>/dev/null | sort -V))
TOTAL=${{#TEST_FILES[@]}}

if [ "$TOTAL" -eq 0 ]; then
    echo -e "${{C_YELLOW}}⚠️  Brak plików z testami w .algo/tests/${{C_RESET}}"
    echo -e "${{C_DIM}}Dodaj testy w panelu AlgoDeck lub wklej pliki .in do .algo/tests/${{C_RESET}}"
    exit 0
fi

printf "${{C_GRAY}}┌───────┬────────────────────────────────┬────────────┬────────────────────────┐${{C_RESET}}\n"
printf "${{C_GRAY}}│${{C_RESET}} ${{C_WHITE}}${{C_BOLD}}%-5s${{C_RESET}} ${{C_GRAY}}│${{C_RESET}} ${{C_WHITE}}${{C_BOLD}}%-30s${{C_RESET}} ${{C_GRAY}}│${{C_RESET}} ${{C_WHITE}}${{C_BOLD}}%10s${{C_RESET}} ${{C_GRAY}}│${{C_RESET}} ${{C_WHITE}}${{C_BOLD}}%-22s${{C_RESET}} ${{C_GRAY}}│${{C_RESET}}\n" "NR" "TEST" "CZAS" "STATUS"
printf "${{C_GRAY}}├───────┼────────────────────────────────┼────────────┼────────────────────────┤${{C_RESET}}\n"

IDX=0
for in_file in "${{TEST_FILES[@]}}"; do
    IDX=$((IDX + 1))
    base=$(basename "$in_file" .in)
    out_file=".algo/tests/$base.out"
    my_out=".algo/tests/$base.my"
    tag_file=".algo/tests/$base.tag"
    tag_text=$(cat "$tag_file" 2>/dev/null || echo "")

    display_name="$base"
    if [ -n "$tag_text" ]; then
        display_name="$base ($tag_text)"
    fi
    if [ ${{#display_name}} -gt 30 ]; then
        display_name="${{display_name:0:27}}..."
    fi

    start=$(date +%s%N)
    ./.algo/${{PROB}} < "$in_file" > "$my_out" 2>/dev/null
    ret=$?
    end=$(date +%s%N)
    diff_ms=$(( (end - start) / 1000000 ))

    raw_status=""
    status_color=""
    is_ok=false

    if [ $ret -ne 0 ]; then
        raw_status="💥 RTE (kod $ret)"
        status_color="${{C_MAGENTA}}${{C_BOLD}}"
        ERRORS+=("$base|RTE|Kod błędu: $ret (program uległ awarii)")
    elif [ "$diff_ms" -gt "$TIME_LIMIT_MS" ] && [ "$TIME_LIMIT_MS" -gt 0 ]; then
        raw_status="⏱️  TLE (>${{TIME_LIMIT_MS}}ms)"
        status_color="${{C_YELLOW}}${{C_BOLD}}"
        ERRORS+=("$base|TLE|Przekroczono limit czasu (${{diff_ms}} ms > ${{TIME_LIMIT_MS}} ms)")
    elif [ -f "$out_file" ] && [ -s "$out_file" ]; then
        if diff -q -w -B "$out_file" "$my_out" >/dev/null 2>&1; then
            raw_status="✓ OK"
            status_color="${{C_GREEN}}${{C_BOLD}}"
            is_ok=true
        else
            raw_status="❌ WA (Zły wynik)"
            status_color="${{C_RED}}${{C_BOLD}}"
            ERRORS+=("$base|WA|diff")
        fi
    else
        raw_status="✓ OK (brak out)"
        status_color="${{C_GREEN}}${{C_BOLD}}"
        is_ok=true
    fi

    if [ "$is_ok" = true ]; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
    fi

    time_str="${{diff_ms}} ms"

    c1=$(pad_right "$IDX" 5)
    c2=$(pad_right "$display_name" 30)
    c3=$(pad_left "$time_str" 10)
    c4=$(pad_right "$raw_status" 22)

    printf "${{C_GRAY}}│${{C_RESET}} %s ${{C_GRAY}}│${{C_RESET}} %s ${{C_GRAY}}│${{C_RESET}} %s ${{C_GRAY}}│${{C_RESET}} ${{status_color}}%s${{C_RESET}} ${{C_GRAY}}│${{C_RESET}}\n" "$c1" "$c2" "$c3" "$c4"

    if [ "$VERBOSE" = true ]; then
        echo -e "${{C_DIM}}--- Wejście ($base.in): ---${{C_RESET}}"
        head -n 10 "$in_file"
        echo -e "${{C_DIM}}--- Wyjście programu: ---${{C_RESET}}"
        head -n 10 "$my_out"
        if [ -f "$out_file" ]; then
            echo -e "${{C_DIM}}--- Oczekiwane wyjście: ---${{C_RESET}}"
            head -n 10 "$out_file"
        fi
        echo ""
    fi
done

printf "${{C_GRAY}}└───────┴────────────────────────────────┴────────────┴────────────────────────┘${{C_RESET}}\n"
echo ""

PCT=0
if [ "$TOTAL" -gt 0 ]; then
    PCT=$(( PASSED * 100 / TOTAL ))
fi

BOX_TOP="╔$(printf "═%.0s" {{1..78}})╗"
BOX_BOT="╚$(printf "═%.0s" {{1..78}})╝"

if [ "$PASSED" -eq "$TOTAL" ]; then
    echo -e "${{C_GREEN}}${{C_BOLD}}${{BOX_TOP}}${{C_RESET}}"
    echo -ne "${{C_GREEN}}${{C_BOLD}}"
    box_line "  ✨ 100% TESTÓW ZALICZONYCH ($PASSED/$TOTAL) — WSZYSTKO POPRAWNIE!"
    echo -e "${{C_GREEN}}${{C_BOLD}}${{BOX_BOT}}${{C_RESET}}"
    notify-send "AlgoDeck (${{PROB_UPPER}})" "✨ 100% ZALICZONE ($PASSED/$TOTAL)" 2>/dev/null || true
elif [ "$PASSED" -eq 0 ]; then
    echo -e "${{C_RED}}${{C_BOLD}}${{BOX_TOP}}${{C_RESET}}"
    echo -ne "${{C_RED}}${{C_BOLD}}"
    box_line "  ❌ 0% TESTÓW ZALICZONYCH (0/$TOTAL) — WSZYSTKIE TESTY OBLANE!"
    echo -e "${{C_RED}}${{C_BOLD}}${{BOX_BOT}}${{C_RESET}}"
    notify-send -u critical "AlgoDeck (${{PROB_UPPER}})" "❌ 0% ZALICZONE (0/$TOTAL)" 2>/dev/null || true
else
    echo -e "${{C_YELLOW}}${{C_BOLD}}${{BOX_TOP}}${{C_RESET}}"
    echo -ne "${{C_YELLOW}}${{C_BOLD}}"
    box_line "  ⚠️  $PCT% TESTÓW ZALICZONYCH ($PASSED/$TOTAL) — WYKRYTO BŁĘDY ($FAILED OBLANE)"
    echo -e "${{C_YELLOW}}${{C_BOLD}}${{BOX_BOT}}${{C_RESET}}"
    notify-send -u critical "AlgoDeck (${{PROB_UPPER}})" "⚠️  $PCT% ZALICZONE ($PASSED/$TOTAL)" 2>/dev/null || true
fi

if [ "${{#ERRORS[@]}}" -gt 0 ] && [ "$VERBOSE" = false ]; then
    echo ""
    echo -e "${{C_RED}}${{C_BOLD}}🔍 SZCZEGÓŁY OBLANYCH TESTÓW:${{C_RESET}}"
    for err in "${{ERRORS[@]}}"; do
        IFS='|' read -r t_name t_type t_info <<< "$err"
        echo -e "${{C_RED}}--------------------------------------------------------------------------------${{C_RESET}}"
        echo -e "${{C_WHITE}}${{C_BOLD}}Test: $t_name [Typ: $t_type]${{C_RESET}}"
        if [ "$t_type" = "WA" ]; then
            out_file=".algo/tests/$t_name.out"
            my_out=".algo/tests/$t_name.my"
            echo -e "${{C_CYAN}}Oczekiwano (wzorzec):${{C_RESET}}"
            head -n 5 "$out_file" | sed 's/^/  /'
            echo -e "${{C_YELLOW}}Otrzymano (Twój program):${{C_RESET}}"
            head -n 5 "$my_out" | sed 's/^/  /'
        else
            echo -e "  $t_info"
        fi
    done
    echo -e "${{C_RED}}--------------------------------------------------------------------------------${{C_RESET}}"
fi

echo ""
echo -e "${{C_GRAY}}💡 Wskazówka: uruchom 'bash .algo/test.sh -v' aby zobaczyć pełne wejścia/wyjścia.${{C_RESET}}"

if [ "$WAIT_AT_END" = true ]; then
    echo ""
    read -p "Naciśnij Enter, aby zakończyć..."
fi
"#,
            p_dir_disp = p_dir.display(),
            p_id = p_id,
            p_upper = p_id.to_uppercase(),
            title = title,
            time_limit_ms = time_limit_ms
        );
        let _ = fs::write(&test_sh_path, test_content);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&test_sh_path, fs::Permissions::from_mode(0o755));
        }

        // 3. kill.sh
        let kill_sh_path = algo_dir.join("kill.sh");
        let kill_content = format!(
            r#"#!/usr/bin/env bash
PROB="{p_id}"
PROB_UPPER="{p_upper}"

pgrep -x "${{PROB}}" 2>/dev/null | while read -r p; do
    kill -9 "$p" 2>/dev/null || true
done

pgrep -f "${{PROB}}/\.algo/run\.sh" 2>/dev/null | while read -r p; do
    [ "$p" != "$$" ] && kill -9 "$p" 2>/dev/null || true
done

pgrep -f "${{PROB}}/\.algo/test\.sh" 2>/dev/null | while read -r p; do
    [ "$p" != "$$" ] && kill -9 "$p" 2>/dev/null || true
done

notify-send "AlgoDeck (${{PROB_UPPER}})" "🛑 Zatrzymano działający program / testy" 2>/dev/null || true
"#,
            p_id = p_id,
            p_upper = p_id.to_uppercase()
        );
        let _ = fs::write(&kill_sh_path, kill_content);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&kill_sh_path, fs::Permissions::from_mode(0o755));
        }

        // 4. accept.sh
        let accept_sh_path = algo_dir.join("accept.sh");
        let accept_content = r#"#!/usr/bin/env bash
TEST_NAME="${1:-}"
if [ -z "$TEST_NAME" ]; then
    echo "Użycie: bash .algo/accept.sh <nazwa_testu> (np. test_1)"
    exit 1
fi
if [ -f ".algo/tests/$TEST_NAME.my" ]; then
    cp ".algo/tests/$TEST_NAME.my" ".algo/tests/$TEST_NAME.out"
    echo "✓ Zaktualizowano wzorzec dla $TEST_NAME wyjściem Twojego programu!"
else
    echo "Błąd: Brak pliku .algo/tests/$TEST_NAME.my (najpierw uruchom testy!)"
fi
"#;
        let _ = fs::write(&accept_sh_path, accept_content);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&accept_sh_path, fs::Permissions::from_mode(0o755));
        }

        // 5. Problem .vscode/tasks.json
        let p_vscode = p_dir.join(".vscode");
        let _ = fs::create_dir_all(&p_vscode);
        let p_tasks_path = p_vscode.join("tasks.json");
        let p_tasks_json = r#"{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "AlgoDeck: ODPAL",
            "type": "shell",
            "command": "bash ${workspaceFolder}/.algo/run.sh",
            "problemMatcher": [],
            "presentation": {
                "reveal": "always",
                "panel": "shared",
                "focus": true,
                "clear": true
            },
            "group": {
                "kind": "build",
                "isDefault": true
            }
        },
        {
            "label": "AlgoDeck: TESTUJ",
            "type": "shell",
            "command": "bash ${workspaceFolder}/.algo/test.sh",
            "problemMatcher": [],
            "presentation": {
                "reveal": "always",
                "panel": "shared",
                "focus": true,
                "clear": true
            },
            "group": {
                "kind": "test",
                "isDefault": true
            }
        }
    ]
}"#;
        let _ = fs::write(p_tasks_path, p_tasks_json);

        // 6. Ensure Root Workspace .vscode/tasks.json
        Self::ensure_root_workspace_tasks(workspace_dir);
    }

    pub fn ensure_root_workspace_tasks(workspace_dir: &Path) {
        let root_vscode = workspace_dir.join(".vscode");
        let _ = fs::create_dir_all(&root_vscode);
        let root_tasks_path = root_vscode.join("tasks.json");
        let root_tasks_json = r#"{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "AlgoDeck: ODPAL",
            "type": "shell",
            "command": "TASK=\"$(cat /tmp/algodeck_active_task.txt 2>/dev/null | tr '[:upper:]' '[:lower:]')\"; [ -z \"$TASK\" ] && TASK=\"$(basename \"${fileDirname}\")\"; [ -f \"${workspaceFolder}/$TASK/.algo/run.sh\" ] && bash \"${workspaceFolder}/$TASK/.algo/run.sh\" || bash \"${fileDirname}/.algo/run.sh\"",
            "problemMatcher": [],
            "presentation": {
                "reveal": "always",
                "panel": "shared",
                "focus": true,
                "clear": true
            },
            "group": {
                "kind": "build",
                "isDefault": true
            }
        },
        {
            "label": "AlgoDeck: TESTUJ",
            "type": "shell",
            "command": "TASK=\"$(cat /tmp/algodeck_active_task.txt 2>/dev/null | tr '[:upper:]' '[:lower:]')\"; [ -z \"$TASK\" ] && TASK=\"$(basename \"${fileDirname}\")\"; [ -f \"${workspaceFolder}/$TASK/.algo/test.sh\" ] && bash \"${workspaceFolder}/$TASK/.algo/test.sh\" || bash \"${fileDirname}/.algo/test.sh\"",
            "problemMatcher": [],
            "presentation": {
                "reveal": "always",
                "panel": "shared",
                "focus": true,
                "clear": true
            },
            "group": {
                "kind": "test",
                "isDefault": true
            }
        }
    ]
}"#;
        let _ = fs::write(root_tasks_path, root_tasks_json);
    }
}
