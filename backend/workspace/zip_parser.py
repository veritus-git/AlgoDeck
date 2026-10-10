import io
import re
import zipfile
from collections import Counter
from pathlib import Path
from typing import Dict, Any, List, Optional, Union

class ZipParser:
    @staticmethod
    def _clean_stem(name: str) -> str:
        """Wyciąga czysty identyfikator testu (np. 'chw/in/chw0a.in' -> 'chw0a')."""
        stem = Path(name).stem
        # Usuń podwójne rozszerzenia jeśli istnieją (np. chw0a.in.txt -> chw0a)
        stem = re.sub(r'\.(in|out|ans|sol)$', '', stem, flags=re.IGNORECASE)
        return stem

    @staticmethod
    def parse_zip(
        zip_source: Union[Path, bytes, io.BytesIO],
        pdf_tests: Optional[List[Dict[str, Any]]] = None
    ) -> Dict[str, Any]:
        """
        Analizuje archiwum ZIP z zestawem testów olimpijskich.
        Wspiera zarówno struktury płaskie, jak i podkatalogi in/ oraz out/ (np. chwocen.zip, akcje_testy.zip).
        """
        if isinstance(zip_source, (bytes, bytearray)):
            zf = zipfile.ZipFile(io.BytesIO(zip_source), "r")
        elif isinstance(zip_source, io.BytesIO):
            zf = zipfile.ZipFile(zip_source, "r")
        else:
            zf = zipfile.ZipFile(str(zip_source), "r")

        pdf_tests = pdf_tests or []

        # Zbierz wszystkie niepuste wpisy niebędące katalogami
        entries = [name for name in zf.namelist() if not name.endswith("/") and not name.startswith("__MACOSX")]

        # Wykryj pliki wejściowe (*.in, in/*)
        in_entries = [
            e for e in entries
            if e.lower().endswith(".in") or "/in/" in e.lower() or "\\in\\" in e.lower()
        ]
        # Jeśli brak z filtrem .in, spróbuj znaleźć pliki z 'in' w nazwie
        if not in_entries:
            in_entries = [e for e in entries if not e.lower().endswith((".out", ".ans", ".sol")) and not "/out/" in e.lower()]

        # Wykryj pliki wyjściowe (*.out, *.ans, *.sol, out/*)
        out_entries = [
            e for e in entries
            if e.lower().endswith((".out", ".ans", ".sol")) or "/out/" in e.lower() or "\\out\\" in e.lower()
        ]

        # Mapa dopasowań wyjść po czystym identyfikatorze
        out_map = {ZipParser._clean_stem(e): e for e in out_entries}

        # Wykryj kod zadania z prefiksu (np. chw0a -> chw, akc1a -> akc)
        prefixes = []
        for e in in_entries:
            stem = ZipParser._clean_stem(e)
            m = re.match(r"^([a-zA-Z]+)", stem)
            if m:
                prefixes.append(m.group(1).lower())

        detected_code = ""
        if prefixes:
            detected_code = Counter(prefixes).most_common(1)[0][0]

        # Sortowanie naturalne (chw0a, chw0b, chw1a, chw2a...)
        def natural_sort_key(s):
            clean = ZipParser._clean_stem(s)
            return [int(text) if text.isdigit() else text.lower() for text in re.split(r'(\d+)', clean)]

        in_entries.sort(key=natural_sort_key)

        tests: List[Dict[str, Any]] = []

        for in_entry in in_entries:
            clean_id = ZipParser._clean_stem(in_entry)
            try:
                in_content = zf.read(in_entry).decode("utf-8", errors="replace")
            except Exception:
                in_content = ""

            out_content = ""
            tag = "[PAKIET TESTÓW ZIP]"

            # 1. Sprawdź, czy archiwum zawiera dedykowany plik wyjściowy
            if clean_id in out_map:
                try:
                    out_content = zf.read(out_map[clean_id]).decode("utf-8", errors="replace").strip()
                    tag = "[PAKIET TESTÓW (WZORZEC .OUT)]"
                except Exception:
                    out_content = ""

            # 2. Jeśli brak w archiwum, dopasuj do testów wyciągniętych z PDF
            if not out_content and pdf_tests:
                norm_in = in_content.strip()
                for pt in pdf_tests:
                    p_in = pt.get("input", "").strip()
                    p_id = pt.get("id", "").lower()
                    if p_in == norm_in or (p_id and p_id in clean_id.lower()) or ("0a" in clean_id and "przykład" in pt.get("name", "").lower()):
                        out_content = pt.get("expected_output", "").strip()
                        tag = "[OFICJALNY Z TREŚCI PDF]"
                        break

            tests.append({
                "id": clean_id,
                "name": clean_id,
                "in_filename": f"{clean_id}.in",
                "out_filename": f"{clean_id}.out" if out_content else "",
                "input": in_content,
                "expected_output": out_content,
                "tag": tag
            })

        return {
            "detected_problem_id": detected_code,
            "tests": tests,
            "raw_entries_count": len(entries)
        }

    @staticmethod
    def parse_directory(
        dir_path: Path,
        pdf_tests: Optional[List[Dict[str, Any]]] = None
    ) -> Dict[str, Any]:
        """
        Analizuje rozpakowany folder z testami (np. /home/linux/Pobrane/Downloads/chw).
        Obsługuje podkatalogi in/ i out/ oraz pliki w jednym katalogu.
        """
        dir_path = Path(dir_path)
        pdf_tests = pdf_tests or []

        all_files = [f for f in dir_path.rglob("*") if f.is_file()]

        in_files = [
            f for f in all_files
            if f.suffix.lower() == ".in" or f.parent.name.lower() == "in"
        ]
        out_files = [
            f for f in all_files
            if f.suffix.lower() in (".out", ".ans", ".sol") or f.parent.name.lower() == "out"
        ]

        out_map = {ZipParser._clean_stem(f.name): f for f in out_files}

        prefixes = []
        for f in in_files:
            stem = ZipParser._clean_stem(f.name)
            m = re.match(r"^([a-zA-Z]+)", stem)
            if m:
                prefixes.append(m.group(1).lower())

        detected_code = ""
        if prefixes:
            detected_code = Counter(prefixes).most_common(1)[0][0]

        def natural_sort_key(p: Path):
            clean = ZipParser._clean_stem(p.name)
            return [int(text) if text.isdigit() else text.lower() for text in re.split(r'(\d+)', clean)]

        in_files.sort(key=natural_sort_key)

        tests: List[Dict[str, Any]] = []

        for in_file in in_files:
            clean_id = ZipParser._clean_stem(in_file.name)
            try:
                in_content = in_file.read_text(encoding="utf-8", errors="replace")
            except Exception:
                in_content = ""

            out_content = ""
            tag = "[FOLDER TESTÓW]"

            if clean_id in out_map:
                try:
                    out_content = out_map[clean_id].read_text(encoding="utf-8", errors="replace").strip()
                    tag = "[FOLDER TESTÓW (WZORZEC .OUT)]"
                except Exception:
                    out_content = ""

            if not out_content and pdf_tests:
                norm_in = in_content.strip()
                for pt in pdf_tests:
                    p_in = pt.get("input", "").strip()
                    p_id = pt.get("id", "").lower()
                    if p_in == norm_in or (p_id and p_id in clean_id.lower()) or ("0a" in clean_id and "przykład" in pt.get("name", "").lower()):
                        out_content = pt.get("expected_output", "").strip()
                        tag = "[OFICJALNY Z TREŚCI PDF]"
                        break

            tests.append({
                "id": clean_id,
                "name": clean_id,
                "in_filename": f"{clean_id}.in",
                "out_filename": f"{clean_id}.out" if out_content else "",
                "input": in_content,
                "expected_output": out_content,
                "tag": tag
            })

        return {
            "detected_problem_id": detected_code,
            "tests": tests,
            "raw_entries_count": len(all_files)
        }

    @staticmethod
    def extract_tests_to_dir(tests: List[Dict[str, Any]], target_dir: Path):
        """Zapisuje wyekstrahowane testy do katalogu .algo/tests/."""
        target_dir.mkdir(parents=True, exist_ok=True)
        for t in tests:
            t_id = t["id"]
            in_file = target_dir / f"{t_id}.in"
            out_file = target_dir / f"{t_id}.out"
            tag_file = target_dir / f"{t_id}.tag"

            in_file.write_text(t.get("input", ""), encoding="utf-8")
            if t.get("expected_output"):
                out_file.write_text(t.get("expected_output", "").strip() + "\n", encoding="utf-8")
            
            tag = t.get("tag", "[TEST]")
            tag_file.write_text(tag, encoding="utf-8")
