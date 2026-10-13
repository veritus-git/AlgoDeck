use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub input: String,
    pub expected_output: String,
    #[serde(default)]
    pub is_edge_case: bool,
    #[serde(default)]
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemAnalysis {
    pub problem_id: String,
    pub title: String,
    pub time_limit_sec: f64,
    pub memory_limit_mb: u64,
    pub tests: Vec<TestCase>,
    pub used_engine: String,
}

pub struct PdfParser;

impl PdfParser {
    pub fn clean_text(text: &str) -> String {
        let mut s = text.to_string();
        // Standaryzacja znaków diakrytycznych i TeX ligatur
        let r_in = Regex::new(r"(?i)Wej[´\'\` ]*scie").unwrap();
        s = r_in.replace_all(&s, "Wejście").to_string();

        let r_out = Regex::new(r"(?i)Wyj[´\'\` ]*scie").unwrap();
        s = r_out.replace_all(&s, "Wyjście").to_string();

        let r_ex = Regex::new(r"(?i)Przyk[´\'\` ]*lad(?:y)?").unwrap();
        s = r_ex.replace_all(&s, "Przykład").to_string();

        let r_expl = Regex::new(r"(?i)W[ ]*yjaśnienie").unwrap();
        s = r_expl.replace_all(&s, "Wyjaśnienie").to_string();

        let r_t = Regex::new(r"(?i)T[ ]*esty").unwrap();
        s = r_t.replace_all(&s, "Testy").to_string();

        let r_tw = Regex::new(r"(?i)Tw[´\'\` ]*oj").unwrap();
        s = r_tw.replace_all(&s, "Twój").to_string();

        let r_bajt = Regex::new(r"(?i)B[ ]*ajtazar").unwrap();
        s = r_bajt.replace_all(&s, "Bajtazar").to_string();

        s
    }

    pub fn extract_text_from_file(path: &Path) -> Result<String, String> {
        let text = pdf_extract::extract_text(path).map_err(|e| format!("Błąd odczytu PDF: {}", e))?;
        Ok(Self::clean_text(&text))
    }

    pub fn parse_heuristics(full_text: &str) -> ProblemAnalysis {
        let lines: Vec<&str> = full_text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

        // 1. Kod zadania
        let mut problem_id = String::new();

        let re_code = Regex::new(r"(?i)Kod zadania:\s*([a-zA-Z0-9_]+)").unwrap();
        if let Some(caps) = re_code.captures(full_text) {
            problem_id = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
        }

        if problem_id.is_empty() {
            let re_file = Regex::new(r"(?i)Plik\s+ź?r[oó]dłowy\s+([a-zA-Z0-9_]+)\.").unwrap();
            if let Some(caps) = re_file.captures(full_text) {
                problem_id = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
            }
        }

        if problem_id.is_empty() {
            let re_task = Regex::new(r"(?i)Zadanie:\s*([a-zA-Z0-9_]{2,10})\b").unwrap();
            if let Some(caps) = re_task.captures(full_text) {
                let candidate = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
                if candidate != "probne" && candidate != "otwarte" && candidate != "glowne" {
                    problem_id = candidate;
                }
            }
        }

        // 2. Tytuł zadania
        let mut title = String::new();
        let re_task_line = Regex::new(r"(?i)^Zadanie:\s*[a-zA-Z0-9_]+").unwrap();
        let re_bad_title = Regex::new(r"(?i)(etap|plik|dostępna|pamięć|punktów)").unwrap();

        for (i, line) in lines.iter().take(12).enumerate() {
            if re_task_line.is_match(line) {
                if let Some(next_line) = lines.get(i + 1) {
                    if !re_bad_title.is_match(next_line) {
                        title = next_line.to_string();
                        break;
                    }
                }
            }
        }

        if title.is_empty() {
            let re_skip = Regex::new(r"(?i)(kod|limit|etap|oij|oi|strona|zawody|punkty|zadanie|www|©)").unwrap();
            for line in lines.iter().take(10) {
                if !re_skip.is_match(line) && line.len() < 45 {
                    title = line.to_string();
                    break;
                }
            }
        }

        if title.is_empty() {
            title = "Zadanie".to_string();
        }

        // Fallback na kod zadania
        if problem_id.is_empty() {
            let slug: String = title.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
            if slug.len() >= 3 {
                problem_id = slug[..3].to_string();
            } else if !slug.is_empty() {
                problem_id = slug;
            } else {
                problem_id = "zad".to_string();
            }
        }

        // 3. Limity
        let mut time_limit = 1.0;
        let re_time = Regex::new(r"(?i)Limit czasu:\s*([0-9]+(?:[\.,][0-9]+)?)\s*s").unwrap();
        if let Some(caps) = re_time.captures(full_text) {
            if let Some(m) = caps.get(1) {
                let s = m.as_str().replace(',', ".");
                if let Ok(val) = s.parse::<f64>() {
                    time_limit = val;
                }
            }
        }

        let mut memory_limit = 128;
        let re_mem = Regex::new(r"(?i)(?:Limit|Dostępna)\s+pamięć:\s*([0-9]+)\s*MB").unwrap();
        if let Some(caps) = re_mem.captures(full_text) {
            if let Some(m) = caps.get(1) {
                if let Ok(val) = m.as_str().parse::<u64>() {
                    memory_limit = val;
                }
            }
        }

        // 4. Testy
        let mut tests: Vec<TestCase> = Vec::new();

        // Wzorzec A: Format OIJ 'Wejście dla testu chw0a:' lub 'Wejście:\n...\nWyjście:\n...'
        let re_oij = Regex::new(
            r"(?is)Wejście\s*(?:dla\s*testu\s*|\s*dla\s*testu)?([a-zA-Z0-9_]*):\s*\n(.*?)\nWyjście\s*(?:dla\s*testu\s*|\s*dla\s*testu)?[a-zA-Z0-9_]*:\s*\n(.*?)(?:\nWejście|\nWyjaśnienie|\nOcenianie|\z)",
        ).unwrap();

        let re_cut = Regex::new(r"(?is)\nWyjaśnienie|\nTesty|\nOcenianie").unwrap();

        for caps in re_oij.captures_iter(full_text) {
            let raw_tag = caps.get(1).map_or("", |m| m.as_str()).trim();
            let test_tag = if raw_tag.is_empty() {
                format!("test_{}", tests.len() + 1)
            } else {
                raw_tag.to_string()
            };
            let inp = caps.get(2).map_or("", |m| m.as_str()).trim();
            let mut out = caps.get(3).map_or("", |m| m.as_str()).trim();
            if let Some(m) = re_cut.find(out) {
                out = &out[..m.start()].trim();
            }

            if !inp.is_empty() && !out.is_empty() {
                tests.push(TestCase {
                    id: test_tag.clone(),
                    name: format!("Przykład {}", test_tag),
                    input: inp.to_string(),
                    expected_output: out.to_string(),
                    is_edge_case: false,
                    tag: "[OFICJALNY Z TREŚCI]".to_string(),
                });
            }
        }

        // Wzorzec B: Format OI 'Dla danych wejściowych:\n...\npoprawnym wynikiem jest:\n...'
        if tests.is_empty() {
            let re_oi = Regex::new(
                r"(?is)Dla\s+danych\s+wejściowych:\s*\n(.*?)\npoprawnym\s+wynikiem\s+jest:\s*\n(.*?)(?:\nWyjaśnienie|\nTesty|\nOcenianie|\z)",
            ).unwrap();

            for (i, caps) in re_oi.captures_iter(full_text).enumerate() {
                let inp = caps.get(1).map_or("", |m| m.as_str()).trim();
                let mut out = caps.get(2).map_or("", |m| m.as_str()).trim();
                if let Some(m) = re_cut.find(out) {
                    out = &out[..m.start()].trim();
                }

                if !inp.is_empty() && !out.is_empty() {
                    tests.push(TestCase {
                        id: format!("test_{}", i + 1),
                        name: format!("Przykład {}", i + 1),
                        input: inp.to_string(),
                        expected_output: out.to_string(),
                        is_edge_case: false,
                        tag: "[OFICJALNY Z TREŚCI]".to_string(),
                    });
                }
            }
        }

        // Wzorzec C: Format Szkopuł / Prosty 'Przykład\nWejście\n...\nWyjście\n...'
        if tests.is_empty() {
            let re_std = Regex::new(
                r"(?is)Przykład(?:y)?\s*\n+Wejście:?\s*\n(.*?)\nWyjście:?\s*\n(.*?)(?:\nWyjaśnienie|\nTesty|\nOcenianie|\nPrzykład|\z)",
            ).unwrap();

            for (i, caps) in re_std.captures_iter(full_text).enumerate() {
                let inp = caps.get(1).map_or("", |m| m.as_str()).trim();
                let mut out = caps.get(2).map_or("", |m| m.as_str()).trim();
                if let Some(m) = re_cut.find(out) {
                    out = &out[..m.start()].trim();
                }

                if !inp.is_empty() && !out.is_empty() {
                    tests.push(TestCase {
                        id: format!("test_{}", i + 1),
                        name: format!("Przykład {}", i + 1),
                        input: inp.to_string(),
                        expected_output: out.to_string(),
                        is_edge_case: false,
                        tag: "[OFICJALNY Z TREŚCI]".to_string(),
                    });
                }
            }
        }

        ProblemAnalysis {
            problem_id,
            title,
            time_limit_sec: time_limit,
            memory_limit_mb: memory_limit,
            tests,
            used_engine: "Lokalny parser zadań olimpijskich MAP/OI (100% offline Rust)".to_string(),
        }
    }
}
