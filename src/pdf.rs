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

        let r_in2 = Regex::new(r"(?i)wej[´\'\` ]*scio").unwrap();
        s = r_in2.replace_all(&s, "wejścio").to_string();

        let r_out = Regex::new(r"(?i)Wyj[´\'\` ]*scie").unwrap();
        s = r_out.replace_all(&s, "Wyjście").to_string();

        let r_out2 = Regex::new(r"(?i)wyj[´\'\` ]*scio").unwrap();
        s = r_out2.replace_all(&s, "wyjścio").to_string();

        let r_pam = Regex::new(r"(?i)pami[´\'\` ]*ec").unwrap();
        s = r_pam.replace_all(&s, "pamięć").to_string();

        let r_dost = Regex::new(r"(?i)dost[´\'\` ]*epn").unwrap();
        s = r_dost.replace_all(&s, "dostępn").to_string();

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

        // 1. Kod zadania i Tytuł (jeśli są w jednej linii np. 'Chwasty Kod zadania: chw')
        let mut problem_id = String::new();
        let mut title = String::new();

        let re_title_code = Regex::new(r"(?i)^(.+?)\s+Kod zadania:\s*([a-zA-Z0-9_]+)").unwrap();
        for line in lines.iter().take(8) {
            if let Some(caps) = re_title_code.captures(line) {
                let cand_title = caps.get(1).map_or("", |m| m.as_str()).trim();
                let cand_id = caps.get(2).map_or("", |m| m.as_str()).to_lowercase();
                if !cand_title.is_empty() && cand_title.len() < 40 && !cand_title.ends_with('.') {
                    title = cand_title.to_string();
                }
                if !cand_id.is_empty() {
                    problem_id = cand_id;
                }
                break;
            }
        }

        if problem_id.is_empty() {
            let re_code = Regex::new(r"(?i)Kod zadania:\s*([a-zA-Z0-9_]+)").unwrap();
            if let Some(caps) = re_code.captures(full_text) {
                problem_id = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
            }
        }

        if problem_id.is_empty() {
            let re_file_cpp = Regex::new(r"(?i)Plik\s+.*?([a-zA-Z0-9_]+)\.(?:cpp|cc|c)\b").unwrap();
            if let Some(caps) = re_file_cpp.captures(full_text) {
                problem_id = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
            }
        }

        if problem_id.is_empty() {
            let re_file = Regex::new(r"(?i)Plik\s+.*?r[oó]d.*?owy:?\s*([a-zA-Z0-9_]+)\.").unwrap();
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

        // 2. Tytuł zadania (jeśli nie wyekstrahowany z linii 'Tytuł Kod zadania: ...')
        if title.is_empty() {
            let re_task_header = Regex::new(r"(?i)^Zadanie:\s*([a-zA-Z0-9ąćęłńóśźżĄĆĘŁŃÓŚŹŻ _-]{2,35})$").unwrap();
            for line in lines.iter().take(12) {
                if let Some(caps) = re_task_header.captures(line) {
                    let cand = caps.get(1).map_or("", |m| m.as_str()).trim();
                    let cand_lower = cand.to_lowercase();
                    if !cand_lower.contains("próbne") && !cand_lower.contains("otwarte") && !cand_lower.contains("zgłoszen") {
                        title = cand.to_string();
                        break;
                    }
                }
            }
        }

        if title.is_empty() {
            let re_skip = Regex::new(r"(?i)(kod|limit|etap|oij|oi|strona|zawody|punkty|zadanie|www|©|zgłoszen|otwart|sprawdz|bajtazar|danych|wejście|wyjście|ocenianie)").unwrap();
            for line in lines.iter().take(10) {
                // Tytuł zadania olimpijskiego to czyste słowo / fraza bez kropki na końcu i bez słów ze stop-listy
                if !re_skip.is_match(line) && line.len() >= 2 && line.len() <= 40 && !line.ends_with('.') && !line.ends_with(':') && !line.ends_with(',') {
                    title = line.to_string();
                    break;
                }
            }
        }

        if title.is_empty() {
            title = if !problem_id.is_empty() {
                let mut c = problem_id.chars();
                match c.next() {
                    None => "Zadanie".to_string(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            } else {
                "Zadanie".to_string()
            };
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
        let re_time = Regex::new(r"(?i)(?:Limit|Dostępny)\s+czasu?:\s*([0-9]+(?:[\.,][0-9]+)?)\s*s").unwrap();
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

        // Wyszukujemy sekcję przykładów, aby nie pomylić wejścia/wyjścia ze specyfikacji zadania
        let re_examples_section = Regex::new(r"(?im)^\s*Przykład(?:y)?\b").unwrap();
        let examples_text = if let Some(m) = re_examples_section.find(full_text) {
            &full_text[m.start()..]
        } else {
            full_text
        };

        // Wzorzec A: Format OIJ 'Wejście dla testu chw0a:' lub 'Wejście:\n...\nWyjście:\n...'
        let re_in_header = Regex::new(r"(?im)^\s*Wejście\s*(?:dla\s*testu\s*|\s*dla\s*testu)?([a-zA-Z0-9_]*):?\s*$").unwrap();
        let re_out_header = Regex::new(r"(?im)^\s*Wyjście\s*(?:dla\s*testu\s*|\s*dla\s*testu)?[a-zA-Z0-9_]*:?\s*$").unwrap();
        let re_cut = Regex::new(r"(?im)^\s*(?:Wyjaśnienie|Testy|Ocenianie|©|Chwasty)").unwrap();

        let in_matches: Vec<(usize, usize, String)> = re_in_header
            .captures_iter(examples_text)
            .filter_map(|c| {
                let m = c.get(0)?;
                let tag = c.get(1).map_or("", |t| t.as_str()).trim().to_string();
                Some((m.start(), m.end(), tag))
            })
            .collect();

        if !in_matches.is_empty() {
            for (idx, &(_start_pos, end_pos, ref tag)) in in_matches.iter().enumerate() {
                let next_test_start = if idx + 1 < in_matches.len() {
                    in_matches[idx + 1].0
                } else {
                    examples_text.len()
                };

                let block = &examples_text[end_pos..next_test_start];
                if let Some(out_cap) = re_out_header.captures(block) {
                    let out_m = out_cap.get(0).unwrap();
                    let input_part = block[..out_m.start()].trim();
                    let mut output_part = block[out_m.end()..].trim();

                    if let Some(cut_m) = re_cut.find(output_part) {
                        output_part = output_part[..cut_m.start()].trim();
                    }

                    if !input_part.is_empty() && !output_part.is_empty() {
                        let test_tag = if tag.is_empty() {
                            format!("test_{}", tests.len() + 1)
                        } else {
                            tag.clone()
                        };
                        tests.push(TestCase {
                            id: test_tag.clone(),
                            name: format!("Przykład {}", test_tag),
                            input: input_part.to_string(),
                            expected_output: output_part.to_string(),
                            is_edge_case: false,
                            tag: "[OFICJALNY Z TREŚCI]".to_string(),
                        });
                    }
                }
            }
        }

        // Wzorzec B: Format OI 'Dla danych wejściowych:...poprawnym wynikiem jest:...'
        if tests.is_empty() {
            let re_oi_combined = Regex::new(r"(?im)^\s*Dla\s+danych\s+wejściowych:?\s+poprawnym\s+wynikiem\s+jest:?\s*$").unwrap();
            let re_oi_in = Regex::new(r"(?im)^\s*Dla\s+danych\s+wejściowych:?").unwrap();
            let re_oi_out = Regex::new(r"(?im)poprawnym\s+wynikiem\s+jest:?").unwrap();

            if let Some(comb_m) = re_oi_combined.find(examples_text) {
                let block = &examples_text[comb_m.end()..];
                let mut cut_end = block.len();
                if let Some(cut_m) = re_cut.find(block) {
                    cut_end = cut_m.start();
                }
                let body = &block[..cut_end];
                let block_lines: Vec<&str> = body.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
                if block_lines.len() >= 2 {
                    let letter_pos = block_lines.iter().position(|l| l.chars().any(|c| c.is_alphabetic()));
                    let split = if let Some(pos) = letter_pos {
                        if pos > 0 { pos } else { block_lines.len() / 2 }
                    } else {
                        block_lines.len() - 1
                    };
                    let in_str = block_lines[..split].join("\n");
                    let out_str = block_lines[split..].join("\n");
                    if !in_str.is_empty() && !out_str.is_empty() {
                        tests.push(TestCase {
                            id: format!("test_{}", tests.len() + 1),
                            name: format!("Przykład {}", tests.len() + 1),
                            input: in_str,
                            expected_output: out_str,
                            is_edge_case: false,
                            tag: "[OFICJALNY Z TREŚCI]".to_string(),
                        });
                    }
                }
            } else {
                let oi_matches: Vec<(usize, usize)> = re_oi_in
                    .captures_iter(examples_text)
                    .filter_map(|c| {
                        let m = c.get(0)?;
                        Some((m.start(), m.end()))
                    })
                    .collect();

                for (idx, &(_start_pos, end_pos)) in oi_matches.iter().enumerate() {
                    let next_test_start = if idx + 1 < oi_matches.len() {
                        oi_matches[idx + 1].0
                    } else {
                        examples_text.len()
                    };

                    let block = &examples_text[end_pos..next_test_start];
                    if let Some(out_cap) = re_oi_out.captures(block) {
                        let out_m = out_cap.get(0).unwrap();
                        let input_part = block[..out_m.start()].trim();
                        let mut output_part = block[out_m.end()..].trim();

                        if let Some(cut_m) = re_cut.find(output_part) {
                            output_part = output_part[..cut_m.start()].trim();
                        }

                        if !input_part.is_empty() && !output_part.is_empty() {
                            tests.push(TestCase {
                                id: format!("test_{}", tests.len() + 1),
                                name: format!("Przykład {}", tests.len() + 1),
                                input: input_part.to_string(),
                                expected_output: output_part.to_string(),
                                is_edge_case: false,
                                tag: "[OFICJALNY Z TREŚCI]".to_string(),
                            });
                        }
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chw_statement() {
        let path = Path::new("/home/linux/AlgoDeckWorkspace/chw/.algo/statement.pdf");
        if path.exists() {
            let text = PdfParser::extract_text_from_file(path).expect("Failed to read statement.pdf");
            let analysis = PdfParser::parse_heuristics(&text);
            assert_eq!(analysis.title, "Chwasty");
            assert_eq!(analysis.problem_id, "chw");
        }
    }

    #[test]
    fn test_parse_koleje_statement() {
        let path = Path::new("frontend/koleje.pdf");
        if path.exists() {
            let text = PdfParser::extract_text_from_file(path).expect("Failed to read koleje.pdf");
            let analysis = PdfParser::parse_heuristics(&text);
            assert_eq!(analysis.problem_id, "kol");
            assert_eq!(analysis.title, "Koleje");
            assert_eq!(analysis.time_limit_sec, 1.0);
            assert_eq!(analysis.memory_limit_mb, 128);
            assert!(!analysis.tests.is_empty(), "Musi wykryć testy z koleje.pdf");
        }
    }
}


