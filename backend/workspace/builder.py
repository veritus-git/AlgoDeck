import json
import logging
import os
import shutil
import subprocess
from pathlib import Path
from typing import Dict, Any, List

from backend.config import settings
from backend.workspace.templates import CPP_TEMPLATE

logger = logging.getLogger("algodeck.workspace")

class WorkspaceBuilder:
    def __init__(self, base_dir: Path = settings.workspace_dir):
        self.base_dir = base_dir
        self.base_dir.mkdir(parents=True, exist_ok=True)

    def create_problem_workspace(self, analysis: Dict[str, Any], original_pdf: Path = None) -> Path:
        """
        Tworzy katalog zadania. W folderze użytkownika znajduje się DOKŁADNIE JEDEN PLIK:
        <problem_id>.cpp
        Wszystkie techniczne pliki (testy, skrypty) są w ukrytym podfolderze .algo/
        """
        problem_id = analysis.get("problem_id", "zad").lower()
        title = analysis.get("title", problem_id)

        problem_dir = self.base_dir / problem_id
        problem_dir.mkdir(parents=True, exist_ok=True)

        # Ukryty katalog techniczny (aby w folderze zadania był tylko jeden plik .cpp)
        algo_hidden_dir = problem_dir / ".algo"
        algo_hidden_dir.mkdir(parents=True, exist_ok=True)
        tests_dir = algo_hidden_dir / "tests"
        tests_dir.mkdir(parents=True, exist_ok=True)

        # 1. JEDYNY WIDOCZNY PLIK: <problem_id>.cpp z czystym kodem i krótkim nagłówkiem
        main_cpp = problem_dir / f"{problem_id}.cpp"
        if not main_cpp.exists():
            cpp_content = (
                CPP_TEMPLATE
                .replace("__TITLE__", str(title))
                .replace("__PROBLEM_ID__", str(problem_id))
                .replace("__TIME_LIMIT__", str(analysis.get("time_limit_sec", 1.0)))
                .replace("__MEMORY_LIMIT__", str(analysis.get("memory_limit_mb", 128)))
            )
            main_cpp.write_text(cpp_content, encoding="utf-8")

        # 2. Zapisz testy do ukrytego katalogu .algo/tests/
        tests: List[Dict[str, Any]] = analysis.get("tests", [])
        saved_tests = []
        for i, test in enumerate(tests, 1):
            t_id = f"test_{i}"
            in_path = tests_dir / f"{t_id}.in"
            out_path = tests_dir / f"{t_id}.out"

            in_path.write_text(test.get("input", "").strip() + "\n", encoding="utf-8")
            out_path.write_text(test.get("expected_output", "").strip() + "\n", encoding="utf-8")

            saved_tests.append({
                "id": t_id,
                "name": test.get("name", f"Test {i}"),
                "in_file": f"{t_id}.in",
                "out_file": f"{t_id}.out"
            })

        # 3. Ukryty test.sh - Pełny log w oknie terminala (Wejście, Wyjście, Diff, Czasy)
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
    notify-send -u critical "AlgoDeck ({problem_id})" "❌ Błąd kompilacji!" 2>/dev/null
    read -p "Naciśnij Enter, aby zamknąć..."
    exit 1
fi
echo -e "\\e[1;32m✓ Skompilowano pomyślnie!\\e[0m"
echo ""

echo -e "\\e[1;33m[2/2] Uruchamianie oficjalnych testów...\\e[0m"
ALL_OK=true
TEST_COUNT=0

for in_file in $(ls -1 .algo/tests/*.in 2>/dev/null | sort -V); do
    [ -e "$in_file" ] || continue
    TEST_COUNT=$((TEST_COUNT + 1))
    base=$(basename "$in_file" .in)
    out_file=".algo/tests/$base.out"
    my_out=".algo/tests/$base.my"
    
    echo -e "\\e[1;34m-----------------------------------------------------\\e[0m"
    echo -e "\\e[1;35m▶ TEST: $base\\e[0m"
    echo -e "\\e[0;36m[DANE WEJŚCIOWE (INPUT)]:\\e[0m"
    cat "$in_file"
    echo ""
    echo -e "\\e[0;33m[ODPALANIE]: ./.algo/{problem_id} < $in_file > $my_out\\e[0m"
    
    start=$(date +%s%N)
    ./.algo/{problem_id} < "$in_file" > "$my_out" 2>/dev/null
    ret=$?
    end=$(date +%s%N)
    diff_ms=$(( (end - start) / 1000000 ))

    if [ $ret -ne 0 ]; then
        echo -e "\\e[1;31m❌ RTE / Crash programu (Kod: $ret, Czas: $diff_ms ms)\\e[0m"
        ALL_OK=false
        continue
    fi

    echo -e "\\e[0;32m[TWOJE WYJŚCIE (OUTPUT)]:\\e[0m"
    cat "$my_out"
    echo ""

    if [ -f "$out_file" ] && [ -s "$out_file" ]; then
        echo -e "\\e[0;34m[OCZEKIWANE WYJŚCIE]:\\e[0m"
        cat "$out_file"
        echo ""
        if diff -q -w -B "$out_file" "$my_out" >/dev/null 2>&1; then
            echo -e "\\e[1;32m✓ WYNIK: OK ($diff_ms ms)\\e[0m"
        else
            echo -e "\\e[1;31m✗ WYNIK: WA (Wrong Answer - niezgodność z wzorcem!)\\e[0m"
            echo -e "\\e[1;31m--- DIFF (--wzorzec ++twoje) ---\\e[0m"
            diff -u -w -B "$out_file" "$my_out" || true
            echo -e "\\e[1;31m--------------------------------\\e[0m"
            ALL_OK=false
        fi
    else
        echo -e "\\e[1;32m✓ WYNIK: OK ($diff_ms ms, brak pliku referencyjnego)\\e[0m"
    fi
done

echo ""
echo -e "\\e[1;34m=====================================================\\e[0m"
if [ "$ALL_OK" = true ]; then
    echo -e "\\e[1;32m🎉 WSZYSTKIE TESTY ZALICZONE (Liczba: $TEST_COUNT)!\\e[0m"
    notify-send "AlgoDeck ({problem_id})" "✅ Wszystkie testy zaliczone!" 2>/dev/null
else
    echo -e "\\e[1;31m❌ ZNALEZIONO BŁĘDY W TESTACH!\\e[0m"
    notify-send -u critical "AlgoDeck ({problem_id})" "❌ Błąd w testach (WA)!" 2>/dev/null
fi
echo ""
read -p "Naciśnij Enter, aby zamknąć okno testów..."
""", encoding="utf-8")
        test_sh.chmod(0o755)

        # 4. Ukryty run.sh
        run_sh = algo_hidden_dir / "run.sh"
        run_sh.write_text(f"""#!/usr/bin/env bash
cd "{problem_dir.resolve()}"

echo -e "\\e[1;36m[AlgoDeck] Kompilacja {problem_id}.cpp...\\e[0m"
if ! g++ -O3 -std=c++20 "{problem_id}.cpp" -o ".algo/{problem_id}"; then
    echo -e "\\e[1;31m❌ Błąd kompilacji!\\e[0m"
    exit 1
fi

echo -e "\\e[1;32m✓ Skompilowano pomyślnie!\\e[0m"
echo -e "\\e[1;33m▶ Program uruchomiony (oczekuje na dane wejściowe / cin):\\e[0m"
echo -e "\\e[0;90m---------------------------------------------------------\\e[0m"

./.algo/{problem_id}
RET=$?

echo ""
echo -e "\\e[0;90m---------------------------------------------------------\\e[0m"
if [ $RET -eq 0 ]; then
    echo -e "\\e[1;32m✓ Program zakończony pomyślnie (kod wyjścia: 0).\\e[0m"
else
    echo -e "\\e[1;31m❌ Program zakończony z błędem (kod wyjścia: $RET).\\e[0m"
fi
""", encoding="utf-8")
        run_sh.chmod(0o755)

        # 5. Ukryty kill.sh
        kill_sh = algo_hidden_dir / "kill.sh"
        kill_sh.write_text(f"""#!/usr/bin/env bash
PROB="{problem_id}"
PROB_UPPER="{problem_id.upper()}"

# 1. Zabij skompilowaną binarkę programu (dokładna nazwa comm: {problem_id})
# NIGDY nie dotyka VS Code ani powłoki systemowej!
pgrep -x "${{PROB}}" 2>/dev/null | while read -r p; do
    comm=$(ps -p "$p" -o comm= 2>/dev/null || true)
    if [ "$comm" = "${{PROB}}" ]; then
        kill -9 "$p" 2>/dev/null || true
    fi
done

# 2. Zabij skrypty wykonawcze run.sh i test.sh dla tego konkretnego zadania
pgrep -f "${{PROB}}/\\.algo/run\\.sh" 2>/dev/null | while read -r p; do
    if [ "$p" != "$$" ]; then
        kill -9 "$p" 2>/dev/null || true
    fi
done

pgrep -f "${{PROB}}/\\.algo/test\\.sh" 2>/dev/null | while read -r p; do
    if [ "$p" != "$$" ]; then
        kill -9 "$p" 2>/dev/null || true
    fi
done

# 3. Jeśli kompilator g++ wisi na błędach lub pętlach szablonów, ubij go
pgrep -f "g\\+\\+.*${{PROB}}" 2>/dev/null | while read -r p; do
    comm=$(ps -p "$p" -o comm= 2>/dev/null || true)
    if [[ "$comm" == *"g++"* || "$comm" == *"cc1plus"* ]]; then
        kill -9 "$p" 2>/dev/null || true
    fi
done

# 4. Zamknij TYLKO dedykowane okno testów (nigdy okno z VS Code)
wmctrl -c "AlgoDeck Testy - ${{PROB_UPPER}}" 2>/dev/null || true

notify-send "AlgoDeck (${{PROB_UPPER}})" "🛑 Zatrzymano działający program / testy" 2>/dev/null || true
""", encoding="utf-8")
        kill_sh.chmod(0o755)

        # 6. Ukryty .vscode/tasks.json do odpalania w terminalu VS Code
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

        # 7. Zapisz manifest w ukrytym katalogu
        manifest = {
            "problem_id": problem_id,
            "title": title,
            "tests": saved_tests,
            "workspace_path": str(problem_dir.resolve())
        }
        (algo_hidden_dir / "problem.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False), encoding="utf-8")

        # 8. Otwórz VS Code
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
            logger.warning(f"Błąd VS Code: {e}")
