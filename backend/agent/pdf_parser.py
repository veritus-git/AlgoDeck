import re
from pathlib import Path
from typing import Dict, Any, List, Optional
import pypdf

class PDFParser:
    @staticmethod
    def extract_text(pdf_path: Path) -> str:
        """Extracts text from all pages of a PDF file using pypdf."""
        reader = pypdf.PdfReader(str(pdf_path))
        full_text = []
        for i, page in enumerate(reader.pages):
            text = page.extract_text() or ""
            full_text.append(f"--- Strona {i+1} ---\n{text}")
        return "\n\n".join(full_text)

    @staticmethod
    def extract_bytes(pdf_path: Path) -> bytes:
        """Reads raw bytes for multimodal Gemini API ingestion."""
        with open(pdf_path, "rb") as f:
            return f.read()

    @staticmethod
    def parse_heuristics(text: str) -> Dict[str, Any]:
        """
        Rule-based heuristic fallback parser for Szkopuł / Olimpiada Informatyczna PDFs.
        Extracts title, short id, limits, and sample tests when offline or without API key.
        """
        lines = [line.strip() for line in text.split("\n") if line.strip()]
        
        # 1. Title extraction
        title = "Zadanie"
        for line in lines[:20]:
            clean = line.replace("--- Strona 1 ---", "").strip()
            if clean and not clean.startswith("Dostępn") and not clean.startswith("Zadanie") and len(clean) < 40:
                title = clean
                break

        # 2. Problem ID extraction (e.g. kol, aut, zap or derived from title)
        problem_id = re.sub(r'[^a-zA-Z0-9]', '', title.lower())[:6] or "zad"
        # Check for explicitly mentioned source filename (np. kol.cpp / kol.c)
        id_match = re.search(r'([a-z0-9_]{2,8})\.(?:cpp|c|pas)', text, re.IGNORECASE)
        if id_match:
            problem_id = id_match.group(1).lower()

        # 3. Limits extraction
        time_limit = 1.0
        time_match = re.search(r'([0-9]+(?:[\.,][0-9]+)?)\s*(?:s|sek|sekund)', text, re.IGNORECASE)
        if time_match:
            try:
                time_limit = float(time_match.group(1).replace(',', '.'))
            except ValueError:
                pass

        memory_limit = 256
        mem_match = re.search(r'([0-9]+)\s*(?:MB|MiB)', text, re.IGNORECASE)
        if mem_match:
            try:
                memory_limit = int(mem_match.group(1))
            except ValueError:
                pass

        # 4. Sample test extraction from Szkopuł / OI format
        tests: List[Dict[str, Any]] = []
        
        # Pattern A: "Dla danych wejściowych: ... poprawnym wynikiem jest: ..."
        pattern_a = re.compile(
            r'(?:Dla danych wejściowych|Wejście):\s*\n?(.*?)(?:poprawnym wynikiem jest|Wyjście):\s*\n?(.*?)(?:Wyjaśnienie|---|\Z)',
            re.DOTALL | re.IGNORECASE
        )
        for i, match in enumerate(pattern_a.finditer(text), 1):
            inp = match.group(1).strip()
            out = match.group(2).strip()
            if inp and out:
                tests.append({
                    "name": f"Przykład {i}",
                    "input": inp,
                    "expected_output": out,
                    "is_edge_case": False,
                    "description": "Oficjalny test przykładowy z treści zadania"
                })

        # Pattern B: Simple column-based or text blocks
        if not tests:
            # Fallback sample test so user is never left with an empty test suite
            tests.append({
                "name": "Przykład 1",
                "input": "5\n1 2 3 4 5",
                "expected_output": "15",
                "is_edge_case": False,
                "description": "Domyślny przykładowy test (zweryfikuj z treścią PDF)"
            })

        # Add synthetic edge cases
        tests.append({
            "name": "Brzegowy: N=1 (Minimum)",
            "input": "1\n1",
            "expected_output": "",
            "is_edge_case": True,
            "description": "Minimalny rozmiar danych wejściowych (N=1)"
        })
        tests.append({
            "name": "Brzegowy: Wartości Graniczne / Zera",
            "input": "3\n0 0 0",
            "expected_output": "",
            "is_edge_case": True,
            "description": "Zerowe lub skrajne wagi elementów"
        })

        return {
            "problem_id": problem_id,
            "title": title,
            "time_limit_sec": time_limit,
            "memory_limit_mb": memory_limit,
            "summary": "Analiza wstępna zadania (ekstrakcja heurystyczna).",
            "input_format": "Format wejścia opisany w pliku PDF.",
            "output_format": "Format wyjścia opisany w pliku PDF.",
            "constraints": "Sprawdź sekcję 'Ograniczenia' w PDF.",
            "tests": tests,
            "recommended_approach": "C++20, szybkie wejście/wyjście."
        }
