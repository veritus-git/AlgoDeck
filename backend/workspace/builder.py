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

        # 1. JEDYNY WIDOCZNY PLIK: <problem_id>.cpp z czystym kodem
        main_cpp = problem_dir / f"{problem_id}.cpp"
        if not main_cpp.exists():
            main_cpp.write_text(CPP_TEMPLATE, encoding="utf-8")

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

        # 3. Ukryty test.sh (skrypt wywoływany przez Stream Deck)
        test_sh = algo_hidden_dir / "test.sh"
        test_sh.write_text(f"""#!/usr/bin/env bash
cd "{problem_dir.resolve()}"
echo -e "\\e[1;36m[AlgoDeck] Kompilacja {problem_id}.cpp...\\e[0m"
g++ -O3 -std=c++20 "{problem_id}.cpp" -o ".algo/{problem_id}" || {{
    notify-send -u critical "AlgoDeck ({problem_id})" "❌ Błąd kompilacji!" 2>/dev/null
    exit 1
}}

ALL_OK=true
echo -e "\\e[1;36m[AlgoDeck] Testowanie...\\e[0m"
for in_file in .algo/tests/*.in; do
    [ -e "$in_file" ] || continue
    base=$(basename "$in_file" .in)
    out_file=".algo/tests/$base.out"
    
    start=$(date +%s%N)
    ./.algo/{problem_id} < "$in_file" > "/tmp/{problem_id}_out.tmp" 2>/dev/null
    ret=$?
    end=$(date +%s%N)
    diff_ms=$(( (end - start) / 1000000 ))

    if [ $ret -ne 0 ]; then
        echo -e "\\e[1;31m[$base] RTE / Crash ($diff_ms ms)\\e[0m"
        ALL_OK=false
        continue
    fi

    if [ -f "$out_file" ] && [ -s "$out_file" ]; then
        if diff -B -w -u "$out_file" "/tmp/{problem_id}_out.tmp" > "/tmp/{problem_id}_diff.tmp"; then
            echo -e "\\e[1;32m[$base] OK ($diff_ms ms)\\e[0m"
        else
            echo -e "\\e[1;31m[$base] WA ($diff_ms ms)\\e[0m"
            cat "/tmp/{problem_id}_diff.tmp" | head -n 8
            ALL_OK=false
        fi
    fi
done

if [ "$ALL_OK" = true ]; then
    notify-send -u normal "AlgoDeck ({problem_id})" "🎉 Wszystkie testy OK!" 2>/dev/null
else
    notify-send -u critical "AlgoDeck ({problem_id})" "❌ Błąd w testach (WA)!" 2>/dev/null
fi
""", encoding="utf-8")
        test_sh.chmod(0o755)

        # 4. Ukryty run.sh
        run_sh = algo_hidden_dir / "run.sh"
        run_sh.write_text(f"""#!/usr/bin/env bash
cd "{problem_dir.resolve()}"
g++ -O3 -std=c++20 "{problem_id}.cpp" -o ".algo/{problem_id}" && ./.algo/{problem_id}
""", encoding="utf-8")
        run_sh.chmod(0o755)

        # 5. Ukryty kill.sh
        kill_sh = algo_hidden_dir / "kill.sh"
        kill_sh.write_text(f"""#!/usr/bin/env bash
pkill -9 -f "{problem_id}" 2>/dev/null
notify-send "AlgoDeck ({problem_id})" "🛑 Zatrzymano proces {problem_id}" 2>/dev/null
""", encoding="utf-8")
        kill_sh.chmod(0o755)

        # 6. Zapisz manifest w ukrytym katalogu
        manifest = {
            "problem_id": problem_id,
            "title": title,
            "tests": saved_tests,
            "workspace_path": str(problem_dir.resolve())
        }
        (algo_hidden_dir / "problem.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False), encoding="utf-8")

        # 7. Otwórz VS Code
        self.open_in_vscode(problem_dir, main_cpp)

        return problem_dir

    def open_in_vscode(self, problem_dir: Path, source_file: Path):
        try:
            if shutil.which("code"):
                subprocess.Popen(
                    ["code", str(problem_dir), str(source_file)],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    start_new_session=True
                )
        except Exception as e:
            logger.warning(f"Błąd VS Code: {e}")
