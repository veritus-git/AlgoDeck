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
        self.generate_test_script(problem_dir, problem_id, title, time_limit)

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

        # 8. .vscode/tasks.json (ODPAL i TESTUJ)
        vscode_dir = problem_dir / ".vscode"
        vscode_dir.mkdir(parents=True, exist_ok=True)
        (vscode_dir / "tasks.json").write_text(json.dumps({
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
                        "focus": True,
                        "clear": True
                    },
                    "group": {
                        "kind": "build",
                        "isDefault": True
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
                        "focus": True,
                        "clear": True
                    },
                    "group": {
                        "kind": "test",
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

    @staticmethod
    def generate_test_script(problem_dir: Path, problem_id: str, title: str = "", time_limit: float = 1.0) -> Path:
        """Generuje czytelny, estetyczny skrypt .algo/test.sh z podsumowaniem i kolorami ANSI."""
        time_limit_ms = int(time_limit * 1000)
        algo_hidden_dir = problem_dir / ".algo"
        algo_hidden_dir.mkdir(parents=True, exist_ok=True)
        test_sh = algo_hidden_dir / "test.sh"

        script_content = f"""#!/usr/bin/env bash
set -o pipefail
cd "{problem_dir.resolve()}"

VERBOSE=false
WAIT_AT_END=false
for arg in "$@"; do
    case "$arg" in
        -v|--verbose|-d|--details) VERBOSE=true ;;
        -w|--wait) WAIT_AT_END=true ;;
    esac
done

PROB="{problem_id}"
PROB_UPPER="{problem_id.upper()}"
PROB_TITLE="{title}"
TIME_LIMIT_MS={time_limit_ms}

C_RESET="\\e[0m"
C_BOLD="\\e[1m"
C_DIM="\\e[2m"
C_GREEN="\\e[1;32m"
C_RED="\\e[1;31m"
C_YELLOW="\\e[1;33m"
C_BLUE="\\e[1;34m"
C_CYAN="\\e[1;36m"
C_MAGENTA="\\e[1;35m"
C_WHITE="\\e[1;37m"
C_GRAY="\\e[0;90m"

echo -e "${{C_CYAN}}${{C_BOLD}}================================================================${{C_RESET}}"
echo -e "${{C_CYAN}}${{C_BOLD}}       ⚡ AlgoDeck Test Runner — ${{PROB_UPPER}} (${{PROB_TITLE}})${{C_RESET}}"
echo -e "${{C_CYAN}}${{C_BOLD}}================================================================${{C_RESET}}"

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

printf "${{C_GRAY}}┌───────┬──────────────────────────────┬────────────┬────────────────────────┐${{C_RESET}}\\n"
printf "${{C_GRAY}}│${{C_RESET}} ${{C_WHITE}}${{C_BOLD}}NR${{C_RESET}}    ${{C_GRAY}}│${{C_RESET}} ${{C_WHITE}}${{C_BOLD}}TEST${{C_RESET}}                         ${{C_GRAY}}│${{C_RESET}} ${{C_WHITE}}${{C_BOLD}}CZAS${{C_RESET}}       ${{C_GRAY}}│${{C_RESET}} ${{C_WHITE}}${{C_BOLD}}STATUS${{C_RESET}}                 ${{C_GRAY}}│${{C_RESET}}\\n"
printf "${{C_GRAY}}├───────┼──────────────────────────────┼────────────┼────────────────────────┤${{C_RESET}}\\n"

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
    if [ ${{#display_name}} -gt 28 ]; then
        display_name="${{display_name:0:25}}..."
    fi

    start=$(date +%s%N)
    ./.algo/${{PROB}} < "$in_file" > "$my_out" 2>/dev/null
    ret=$?
    end=$(date +%s%N)
    diff_ms=$(( (end - start) / 1000000 ))

    status_str=""
    is_ok=false

    if [ $ret -ne 0 ]; then
        status_str="${{C_MAGENTA}}${{C_BOLD}}💥 RTE (kod $ret)${{C_RESET}}"
        ERRORS+=("$base|RTE|Kod błędu: $ret (program uległ awarii)")
    elif [ "$diff_ms" -gt "$TIME_LIMIT_MS" ] && [ "$TIME_LIMIT_MS" -gt 0 ]; then
        status_str="${{C_YELLOW}}${{C_BOLD}}⏱️  TLE (>${{TIME_LIMIT_MS}}ms)${{C_RESET}}"
        ERRORS+=("$base|TLE|Przekroczono limit czasu (${{diff_ms}} ms > ${{TIME_LIMIT_MS}} ms)")
    elif [ -f "$out_file" ] && [ -s "$out_file" ]; then
        if diff -q -w -B "$out_file" "$my_out" >/dev/null 2>&1; then
            status_str="${{C_GREEN}}${{C_BOLD}}✓ OK${{C_RESET}}"
            is_ok=true
        else
            status_str="${{C_RED}}${{C_BOLD}}❌ WA (Zły wynik)${{C_RESET}}"
            ERRORS+=("$base|WA|diff")
        fi
    else
        status_str="${{C_GREEN}}${{C_BOLD}}✓ OK (brak out)${{C_RESET}}"
        is_ok=true
    fi

    if [ "$is_ok" = true ]; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
    fi

    time_display=$(printf "%4d ms" "$diff_ms")
    printf "${{C_GRAY}}│${{C_RESET}} %-5d ${{C_GRAY}}│${{C_RESET}} %-28s ${{C_GRAY}}│${{C_RESET}} %-10s ${{C_GRAY}}│${{C_RESET}} %-32b ${{C_GRAY}}│${{C_RESET}}\\n" "$IDX" "$display_name" "$time_display" "$status_str"

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

printf "${{C_GRAY}}└───────┴──────────────────────────────┴────────────┴────────────────────────┘${{C_RESET}}\\n"
echo ""

PCT=0
if [ "$TOTAL" -gt 0 ]; then
    PCT=$(( PASSED * 100 / TOTAL ))
fi

if [ "$PASSED" -eq "$TOTAL" ]; then
    echo -e "${{C_GREEN}}${{C_BOLD}}╔════════════════════════════════════════════════════════════════╗${{C_RESET}}"
    echo -e "${{C_GREEN}}${{C_BOLD}}║  ✨ 100% TESTÓW ZALICZONYCH ($PASSED/$TOTAL) — WSZYSTKO POPRAWNIE!       ║${{C_RESET}}"
    echo -e "${{C_GREEN}}${{C_BOLD}}╚════════════════════════════════════════════════════════════════╝${{C_RESET}}"
    notify-send "AlgoDeck (${{PROB_UPPER}})" "✨ 100% ZALICZONE ($PASSED/$TOTAL)" 2>/dev/null || true
elif [ "$PASSED" -eq 0 ]; then
    echo -e "${{C_RED}}${{C_BOLD}}╔════════════════════════════════════════════════════════════════╗${{C_RESET}}"
    echo -e "${{C_RED}}${{C_BOLD}}║  ❌ 0% TESTÓW ZALICZONYCH (0/$TOTAL) — WSZYSTKIE TESTY OBLANE!     ║${{C_RESET}}"
    echo -e "${{C_RED}}${{C_BOLD}}╚════════════════════════════════════════════════════════════════╝${{C_RESET}}"
    notify-send -u critical "AlgoDeck (${{PROB_UPPER}})" "❌ 0% ZALICZONE (0/$TOTAL)" 2>/dev/null || true
else
    echo -e "${{C_YELLOW}}${{C_BOLD}}╔════════════════════════════════════════════════════════════════╗${{C_RESET}}"
    printf "${{C_YELLOW}}${{C_BOLD}}║  ⚠️  %2d%% TESTÓW ZALICZONYCH (%d/%d) — WYKRYTO BŁĘDY (%d OBLANE)    ║${{C_RESET}}\\n" "$PCT" "$PASSED" "$TOTAL" "$FAILED"
    echo -e "${{C_YELLOW}}${{C_BOLD}}╚════════════════════════════════════════════════════════════════╝${{C_RESET}}"
    notify-send -u critical "AlgoDeck (${{PROB_UPPER}})" "⚠️  $PCT% ZALICZONE ($PASSED/$TOTAL)" 2>/dev/null || true
fi

if [ "${{#ERRORS[@]}}" -gt 0 ] && [ "$VERBOSE" = false ]; then
    echo ""
    echo -e "${{C_RED}}${{C_BOLD}}🔍 SZCZEGÓŁY OBLANYCH TESTÓW:${{C_RESET}}"
    for err in "${{ERRORS[@]}}"; do
        IFS='|' read -r t_name t_type t_info <<< "$err"
        echo -e "${{C_RED}}----------------------------------------------------------------${{C_RESET}}"
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
    echo -e "${{C_RED}}----------------------------------------------------------------${{C_RESET}}"
fi

echo ""
echo -e "${{C_GRAY}}💡 Wskazówka: uruchom 'bash .algo/test.sh -v' aby zobaczyć pełne wejścia/wyjścia.${{C_RESET}}"

if [ "$WAIT_AT_END" = true ]; then
    echo ""
    read -p "Naciśnij Enter, aby zakończyć..."
fi
"""
        test_sh.write_text(script_content, encoding="utf-8")
        test_sh.chmod(0o755)
        return test_sh
