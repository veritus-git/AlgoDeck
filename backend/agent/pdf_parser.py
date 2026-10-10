import re
import unicodedata
from pathlib import Path
from typing import Dict, Any, List
import pypdf

class PDFParser:
    @staticmethod
    def clean_text(text: str) -> str:
        """
        Normalizuje tekst z dokumentów PDF generowanych przez LaTeX/TeX.
        Naprawia spacje w akcentach i ligaturach specyficznych dla zadań OI, OIJ, MAP i Szkopuła.
        """
        # Standaryzacja znaków diakrytycznych
        text = re.sub(r'Wej[´\'\` ]*scie', 'Wejście', text, flags=re.IGNORECASE)
        text = re.sub(r'Wyj[´\'\` ]*scie', 'Wyjście', text, flags=re.IGNORECASE)
        text = re.sub(r'Przyk[´\'\` ]*lad(?:y)?', 'Przykład', text, flags=re.IGNORECASE)
        text = re.sub(r'W[ ]*yjaśnienie', 'Wyjaśnienie', text, flags=re.IGNORECASE)
        text = re.sub(r'W[ ]*yjaśnienie', 'Wyjaśnienie', text, flags=re.IGNORECASE)
        text = re.sub(r'T[ ]*esty', 'Testy', text, flags=re.IGNORECASE)
        text = re.sub(r'Tw[´\'\` ]*oj', 'Twój', text, flags=re.IGNORECASE)
        text = re.sub(r'Zdo[ ]*la[ ]*l[ ]*ju[˙\`\' ]*z', 'Zdołał już', text, flags=re.IGNORECASE)
        text = re.sub(r'B[ ]*ajtazar', 'Bajtazar', text, flags=re.IGNORECASE)
        return text

    @staticmethod
    def extract_text(pdf_path: Path) -> str:
        """Wyodrębnia i czyści pełny tekst ze wszystkich stron pliku PDF."""
        reader = pypdf.PdfReader(str(pdf_path))
        full_text = []
        for i, page in enumerate(reader.pages):
            page_text = page.extract_text() or ""
            cleaned = PDFParser.clean_text(page_text)
            full_text.append(f"--- Strona {i+1} ---\n{cleaned}")
        return "\n\n".join(full_text)

    @staticmethod
    def parse_heuristics(text: str) -> Dict[str, Any]:
        """
        W 100% lokalny, deterministyczny parser dla zadań z Koła MAP, OIJ, OI oraz Szkopuła.
        Nie wymaga AI ani połączenia sieciowego.
        """
        # Usuń znaczniki stron na potrzeby wyszukiwania
        clean_full = re.sub(r'---\s*Strona\s*\d+\s*---\n?', '', text)
        lines = [line.strip() for line in clean_full.split("\n") if line.strip()]

        # 1. Kod zadania (krótki identyfikator, np. 'chw', 'sil', 'akc')
        problem_id = ""
        # Wzorzec A: Kod zadania: chw
        code_match = re.search(r'Kod zadania:\s*([a-zA-Z0-9_]+)', clean_full, re.IGNORECASE)
        if code_match:
            problem_id = code_match.group(1).lower()

        # Wzorzec B: Plik źródłowy sil.* lub sil.cpp
        if not problem_id:
            file_match = re.search(r'Plik\s+ź?r[oó]dłowy\s+([a-zA-Z0-9_]+)\.', clean_full, re.IGNORECASE)
            if file_match:
                problem_id = file_match.group(1).lower()

        # Wzorzec C: Zadanie: SIL
        if not problem_id:
            task_match = re.search(r'Zadanie:\s*([a-zA-Z0-9_]{2,10})\b', clean_full, re.IGNORECASE)
            if task_match:
                candidate = task_match.group(1).lower()
                if candidate not in ("probne", "otwarte", "glowne"):
                    problem_id = candidate

        # 2. Tytuł zadania
        title = ""
        # Sprawdź, czy po 'Zadanie: KOD' w kolejnej linii jest pełny tytuł (np. 'Siłownia dla początkujących')
        for i, line in enumerate(lines[:12]):
            if re.match(r'^Zadanie:\s*[a-zA-Z0-9_]+', line, re.IGNORECASE):
                if i + 1 < len(lines):
                    next_line = lines[i + 1]
                    if not re.search(r'(etap|plik|dostępna|pamięć|punktów)', next_line, re.IGNORECASE):
                        title = next_line
                        break
        
        # Jeśli brak, weź pierwszą linię która nie jest nagłówkiem technicznym
        if not title:
            for line in lines[:10]:
                if not re.search(r'(kod|limit|etap|oij|oi|strona|zawody|punkty|zadanie|www|©)', line, re.IGNORECASE) and len(line) < 45:
                    title = line
                    break

        title = title or "Zadanie"

        # Fallback na kod zadania jeśli nie znaleziono
        if not problem_id:
            # Tworzymy slug z pierwszych 3-4 znaków tytułu
            slug = re.sub(r'[^a-zA-Z0-9]', '', title.lower())
            problem_id = slug[:3] if len(slug) >= 3 else (slug or "zad")

        # 3. Limity
        time_limit = 1.0
        time_match = re.search(r'Limit czasu:\s*([0-9]+(?:[\.,][0-9]+)?)\s*s', clean_full, re.IGNORECASE)
        if time_match:
            try:
                time_limit = float(time_match.group(1).replace(',', '.'))
            except ValueError:
                pass

        memory_limit = 128
        mem_match = re.search(r'(?:Limit|Dostępna)\s+pamięć:\s*([0-9]+)\s*MB', clean_full, re.IGNORECASE)
        if mem_match:
            try:
                memory_limit = int(mem_match.group(1))
            except ValueError:
                pass

        # 4. Ekstrakcja oficjalnych przykładów
        tests: List[Dict[str, Any]] = []

        # Wzorzec A: Format OIJ 'Wejście dla testu chw0a:' lub 'Wejście dla testuchw0a:'
        pat_oij = re.compile(
            r'Wejście\s*(?:dla\s*testu\s*|\s*dla\s*testu)?([a-zA-Z0-9_]*):\s*\n(.*?)\nWyjście\s*(?:dla\s*testu\s*|\s*dla\s*testu)?[a-zA-Z0-9_]*:\s*\n(.*?)(?=\nWejście|\nWyjaśnienie|\nOcenianie|\Z)',
            re.DOTALL | re.IGNORECASE
        )
        for match in pat_oij.finditer(clean_full):
            test_tag = match.group(1).strip() or f"test_{len(tests) + 1}"
            inp = match.group(2).strip()
            out = match.group(3).strip()
            # Obetnij ewentualne doklejone wyjaśnienie
            out = re.split(r'\nWyjaśnienie|\nTesty|\nOcenianie', out, flags=re.IGNORECASE)[0].strip()
            if inp and out:
                tests.append({
                    "id": test_tag,
                    "name": f"Przykład {test_tag}",
                    "input": inp,
                    "expected_output": out,
                    "is_edge_case": False,
                    "tag": "[OFICJALNY Z TREŚCI]"
                })

        # Wzorzec B: Format OI 'Dla danych wejściowych:\n...\npoprawnym wynikiem jest:\n...'
        if not tests:
            pat_oi = re.compile(
                r'Dla\s+danych\s+wejściowych:\s*\n(.*?)\npoprawnym\s+wynikiem\s+jest:\s*\n(.*?)(?=\nWyjaśnienie|\nTesty|\nOcenianie|\Z)',
                re.DOTALL | re.IGNORECASE
            )
            for i, match in enumerate(pat_oi.finditer(clean_full), 1):
                inp = match.group(1).strip()
                out = match.group(2).strip()
                out = re.split(r'\nWyjaśnienie|\nTesty|\nOcenianie', out, flags=re.IGNORECASE)[0].strip()
                if inp and out:
                    tests.append({
                        "id": f"test_{i}",
                        "name": f"Przykład {i}",
                        "input": inp,
                        "expected_output": out,
                        "is_edge_case": False,
                        "tag": "[OFICJALNY Z TREŚCI]"
                    })

        # Wzorzec C: Format standardowy Szkopuł / Prosty 'Przykład\nWejście\n...\nWyjście\n...'
        if not tests:
            pat_std = re.compile(
                r'Przykład(?:y)?\s*\n+Wejście:?\s*\n(.*?)\nWyjście:?\s*\n(.*?)(?=\nWyjaśnienie|\nTesty|\nOcenianie|\nPrzykład|\Z)',
                re.DOTALL | re.IGNORECASE
            )
            for i, match in enumerate(pat_std.finditer(clean_full), 1):
                inp = match.group(1).strip()
                out = match.group(2).strip()
                out = re.split(r'\nWyjaśnienie|\nTesty|\nOcenianie', out, flags=re.IGNORECASE)[0].strip()
                if inp and out:
                    tests.append({
                        "id": f"test_{i}",
                        "name": f"Przykład {i}",
                        "input": inp,
                        "expected_output": out,
                        "is_edge_case": False,
                        "tag": "[OFICJALNY Z TREŚCI]"
                    })

        return {
            "problem_id": problem_id,
            "title": title,
            "time_limit_sec": time_limit,
            "memory_limit_mb": memory_limit,
            "tests": tests,
            "used_engine": "Lokalny parser zadań olimpijskich MAP/OI (100% offline)"
        }
