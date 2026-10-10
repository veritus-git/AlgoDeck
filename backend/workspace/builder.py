import json
import logging
import os
import re
import shutil
import subprocess
from pathlib import Path
from typing import Dict, Any, List, Optional

from backend.config import settings
from backend.workspace.templates import CPP_TEMPLATE

logger = logging.getLogger("algodeck.workspace")

class WorkspaceBuilder:
    def __init__(self, base_dir: Path = settings.workspace_dir):
        self.base_dir = base_dir
        self.base_dir.mkdir(parents=True, exist_ok=True)

    def create_problem_workspace(self, analysis: Dict[str, Any], original_pdf: Optional[Path] = None) -> Path:
        """
        Tworzy profesjonalny workspace olimpijski dla zadania.
        W głównym folderze znajduje się DOKŁADNIE JEDEN PLIK:
        <problem_id>.cpp
        Wszystkie pliki techniczne (testy, skrypty wykonawcze) są w ukrytym podfolderze .algo/
        """
        problem_id = analysis.get("problem_id", "zad").lower().strip()
        title = analysis.get("title", problem_id).strip()
        time_limit = analysis.get("time_limit_sec", 1.0)
        memory_limit = analysis.get("memory_limit_mb", 128)

        problem_dir = self.base_dir / problem_id
        problem_dir.mkdir(parents=True, exist_ok=True)

        # Ukryty katalog techniczny
        algo_hidden_dir = problem_dir / ".algo"
        algo_hidden_dir.mkdir(parents=True, exist_ok=True)
        tests_dir = algo_hidden_dir / "tests"
        tests_dir.mkdir(parents=True, exist_ok=True)
        # Wyczyść stare testy jeśli to ponowny import tego samego zadania
        for old_f in tests_dir.glob("*"):
            if old_f.is_file():
                old_f.unlink(missing_ok=True)

        # 1. Główny plik rozwiązania C++20: <problem_id>.cpp
        main_cpp = problem_dir / f"{problem_id}.cpp"
        if not main_cpp.exists():
            cpp_content = (
                CPP_TEMPLATE
                .replace("__TITLE__", str(title))
                .replace("__PROBLEM_ID__", str(problem_id))
                .replace("__TIME_LIMIT__", str(time_limit))
                .replace("__MEMORY_LIMIT__", str(memory_limit))
            )
            main_cpp.write_text(cpp_content, encoding="utf-8")

        # 2. Skopiuj treść PDF jeśli podano
        if original_pdf and original_pdf.exists():
            target_pdf = algo_hidden_dir / "statement.pdf"
            try:
                shutil.copy2(original_pdf, target_pdf)
            except Exception as e:
                logger.warning(f"Nie udało się skopiować pliku PDF: {e}")

        # 3. Zapisz testy w .algo/tests/
        tests: List[Dict[str, Any]] = analysis.get("tests", [])
        saved_tests = []
        for i, test in enumerate(tests, 1):
            t_id = test.get("id") or f"test_{i}"
            # Oczyść id z niedozwolonych znaków
            t_id = re.sub(r'[^a-zA-Z0-9_-]', '', t_id) or f"test_{i}"

            in_path = tests_dir / f"{t_id}.in"
            out_path = tests_dir / f"{t_id}.out"
            tag_path = tests_dir / f"{t_id}.tag"

            in_content = test.get("input", "")
            out_content = test.get("expected_output", "").strip()
            tag = test.get("tag") or ("[OFICJALNY Z TREŚCI]" if out_content else "[PAKIET TESTÓW]")

            in_path.write_text(in_content.strip() + "\n" if in_content else "\n", encoding="utf-8")
            if out_content:
                out_path.write_text(out_content + "\n", encoding="utf-8")
            tag_path.write_text(tag, encoding="utf-8")

            saved_tests.append({
                "id": t_id,
                "name": test.get("name", t_id),
                "in_file": f"{t_id}.in",
                "out_file": f"{t_id}.out" if out_content else "",
                "tag": tag
            })

        # 4. Skrypt test.sh uruchamiany przez przycisk TESTUJ
        test_sh = algo_hidden_dir / "test.sh"
        test_sh.write_text(f"""#!/usr/bin/env bash
cd "{problem_dir.resolve()}"

echo -e "\\e[1;36m=====================================================\\e[0m"
echo -e "\\e[1;36m       🚀 AlgoDeck Test Runner - {problem_id.upper()} ({title})\\e[0m"
echo -e "\\e[1;36m=====================================================\\e[0m"
echo ""

echo -e "\\e[1;33m[1/2] Kompilacja {problem_id}.cpp (g++ -O3 -std=c++20)...\\e[0m"
if ! g++ -O3 -std=c++20 "{problem_id}.cpp" -o ".algo/{problem_id}"; then
    echo -e "\\e[1;31m❌ BŁĄD KOMPILACJI!\\e[0m"
    notify-send -u critical "AlgoDeck ({problem_id})" "❌ Błąd kompilacji!" 2>/dev/null || true
    read -p "Naciśnij Enter, aby zamknąć..."
    exit 1
fi
echo -e "\\e[1;32m✓ Skompilowano pomyślnie!\\e[0m"
echo ""

echo -e "\\e[1;33m[2/2] Uruchamianie testów...\\e[0m"
ALL_OK=true
TEST_COUNT=0

for in_file in $(ls -1 .algo/tests/*.in 2>/dev/null | sort -V); do
    [ -e "$in_file" ] || continue
    TEST_COUNT=$((TEST_COUNT + 1))
    base=$(basename "$in_file" .in)
    out_file=".algo/tests/$base.out"
    my_out=".algo/tests/$base.my"
    tag_file=".algo/tests/$base.tag"
    tag_text=$(cat "$tag_file" 2>/dev/null || echo "")
    
    echo -e "\\e[1;34m-----------------------------------------------------\\e[0m"
    echo -e "\\e[1;35m▶ TEST: $base \\e[0;90m$tag_text\\e[0m"
    
    # Podgląd pierwszych linii wejścia
    lines_count=$(wc -l < "$in_file")
    if [ "$lines_count" -le 10 ]; then
        echo -e "\\e[0;36m[WEJŚCIE]:\\e[0m"
        cat "$in_file"
    else
        echo -e "\\e[0;36m[WEJŚCIE (duży test - $lines_count linii, podgląd pierwszych 5)]:\\e[0m"
        head -n 5 "$in_file"
    fi
    echo ""
    
    start=$(date +%s%N)
    ./.algo/{problem_id} < "$in_file" > "$my_out" 2>/dev/null
    ret=$?
    end=$(date +%s%N)
    diff_ms=$(( (end - start) / 1000000 ))

    if [ $ret -ne 0 ]; then
        echo -e "\\e[1;31m❌ RTE / Crash programu (Kod wyjścia: $ret, Czas: $diff_ms ms)\\e[0m"
        ALL_OK=false
        continue
    fi

    out_lines=$(wc -l < "$my_out")
    if [ "$out_lines" -le 10 ]; then
        echo -e "\\e[0;32m[TWOJE WYJŚCIE]:\\e[0m"
        cat "$my_out"
    else
        echo -e "\\e[0;32m[TWOJE WYJŚCIE ($out_lines linii, pierwsze 5)]:\\e[0m"
        head -n 5 "$my_out"
    fi
    echo ""

    if [ -f "$out_file" ] && [ -s "$out_file" ]; then
        if diff -q -w -B "$out_file" "$my_out" >/dev/null 2>&1; then
            echo -e "\\e[1;32m✓ WYNIK: OK ($diff_ms ms)\\e[0m"
        else
            echo -e "\\e[1;31m✗ WYNIK: WA (Wrong Answer - niezgodność z wzorcem!)\\e[0m"
            echo -e "\\e[1;31m--- DIFF (--wzorzec ++twoje) ---\\e[0m"
            diff -u -w -B "$out_file" "$my_out" | head -n 25 || true
            echo -e "\\e[1;31m--------------------------------\\e[0m"
            ALL_OK=false
        fi
    else
        echo -e "\\e[1;32m✓ WYNIK: OK ($diff_ms ms, wykonano poprawnie)\\e[0m"
    fi
done

echo ""
echo -e "\\e[1;34m=====================================================\\e[0m"
if [ "$ALL_OK" = true ]; then
    echo -e "\\e[1;32m🎉 WSZYSTKIE TESTY ZALICZONE (Liczba: $TEST_COUNT)!\\e[0m"
    notify-send "AlgoDeck ({problem_id})" "✅ Wszystkie testy zaliczone!" 2>/dev/null || true
else
    echo -e "\\e[1;31m❌ ZNALEZIONO BŁĘDY W TESTACH!\\e[0m"
    notify-send -u critical "AlgoDeck ({problem_id})" "❌ Błąd w testach (WA / RTE)!" 2>/dev/null || true
fi
echo ""
read -p "Naciśnij Enter, aby zamknąć okno testów..."
""", encoding="utf-8")
        test_sh.chmod(0o755)

        # 5. Skrypt accept.sh do szybkiej akceptacji własnego wyniku
        accept_sh = algo_hidden_dir / "accept.sh"
        accept_sh.write_text("""#!/usr/bin/env bash
TEST_NAME="${1:-}"
if [ -z "$TEST_NAME" ]; then
    echo "Użycie: bash .algo/accept.sh <nazwa_testu> (np. akc1a)"
    exit 1
fi
if [ -f ".algo/tests/$TEST_NAME.my" ]; then
    cp ".algo/tests/$TEST_NAME.my" ".algo/tests/$TEST_NAME.out"
    echo "✓ Zaktualizowano wzorzec dla $TEST_NAME wyjściem Twojego programu!"
else
    echo "Błąd: Brak pliku .algo/tests/$TEST_NAME.my (najpierw uruchom testy!)"
fi
""", encoding="utf-8")
        accept_sh.chmod(0o755)

        # 6. Skrypt run.sh do interaktywnego odpalania w terminalu VS Code
        run_sh = algo_hidden_dir / "run.sh"
        run_sh.write_text(f"""#!/usr/bin/env bash
cd "{problem_dir.resolve()}"

echo -e "\\e[1;36m[AlgoDeck] Kompilacja {problem_id}.cpp...\\e[0m"
if ! g++ -O3 -std=c++20 "{problem_id}.cpp" -o ".algo/{problem_id}"; then
    echo -e "\\e[1;31m❌ Błąd kompilacji!\\e[0m"
    exit 1
fi

echo -e "\\e[1;32m✓ Skompilowano pomyślnie!\\e[0m"
echo -e "\\e[1;33m▶ Program uruchomiony (wprowadź dane cin):\\e[0m"
echo -e "\\e[0;90m---------------------------------------------------------\\e[0m"

RET=0
./.algo/{problem_id}
RET=$?

echo ""
echo -e "\\e[0;90m---------------------------------------------------------\\e[0m"
if [ "${{RET:-0}}" -eq 0 ]; then
    echo -e "\\e[1;32m✓ Program zakończony pomyślnie (kod: 0).\\e[0m"
else
    echo -e "\\e[1;31m❌ Program zakończony błędem (kod: ${{RET}}).\\e[0m"
fi
""", encoding="utf-8")
        run_sh.chmod(0o755)

        # 7. Skrypt kill.sh do natychmiastowego ubicia procesów
        kill_sh = algo_hidden_dir / "kill.sh"
        kill_sh.write_text(f"""#!/usr/bin/env bash
PROB="{problem_id}"
PROB_UPPER="{problem_id.upper()}"

pgrep -x "${{PROB}}" 2>/dev/null | while read -r p; do
    kill -9 "$p" 2>/dev/null || true
done

pgrep -f "${{PROB}}/\\.algo/run\\.sh" 2>/dev/null | while read -r p; do
    [ "$p" != "$$" ] && kill -9 "$p" 2>/dev/null || true
done

pgrep -f "${{PROB}}/\\.algo/test\\.sh" 2>/dev/null | while read -r p; do
    [ "$p" != "$$" ] && kill -9 "$p" 2>/dev/null || true
done

wmctrl -c "AlgoDeck Testy - ${{PROB_UPPER}}" 2>/dev/null || true
notify-send "AlgoDeck (${{PROB_UPPER}})" "🛑 Zatrzymano działający program / testy" 2>/dev/null || true
""", encoding="utf-8")
        kill_sh.chmod(0o755)

        # 8. .vscode/tasks.json
        vscode_dir = problem_dir / ".vscode"
        vscode_dir.mkdir(parents=True, exist_ok=True)
        (vscode_dir / "tasks.json").write_text(json.dumps({
            "version": "2.0.0",
            "tasks": [
                {
                    "label": "AlgoDeck: ODPAL",
                    "type": "shell",
                    "command": f"bash ${{workspaceFolder}}/.algo/run.sh",
                    "problemMatcher": [],
                    "presentation": {
                        "reveal": "always",
                        "panel": "shared",
                        "focus": True,
                        "clear": True
                    },
                    "group": {
                        "kind": "build",
                        "isDefault": True
                    }
                }
            ]
        }, indent=4), encoding="utf-8")

        # 9. Manifest problem.json
        manifest = {
            "problem_id": problem_id,
            "title": title,
            "time_limit_sec": time_limit,
            "memory_limit_mb": memory_limit,
            "tests": saved_tests,
            "workspace_path": str(problem_dir.resolve())
        }
        (algo_hidden_dir / "problem.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False), encoding="utf-8")

        # 10. Otwórz VS Code
        self.open_in_vscode(problem_dir, main_cpp)

        return problem_dir

    def open_in_vscode(self, problem_dir: Path, source_file: Path):
        try:
            sd_code_sh = Path(os.path.expanduser("~/.local/bin/sd_algo_code.sh"))
            if sd_code_sh.exists():
                subprocess.Popen(
                    [str(sd_code_sh), str(problem_dir), str(source_file)],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    start_new_session=True
                )
            elif shutil.which("code"):
                subprocess.Popen(
                    ["code", str(problem_dir), str(source_file)],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    start_new_session=True
                )
        except Exception as e:
            logger.warning(f"Błąd uruchamiania VS Code: {e}")
