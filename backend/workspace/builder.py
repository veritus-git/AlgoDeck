import json
import logging
import os
import shutil
import subprocess
from pathlib import Path
from typing import Dict, Any, List

from backend.config import settings
from backend.workspace.templates import (
    CPP_TEMPLATE,
    BRUTE_CPP_TEMPLATE,
    GEN_PY_TEMPLATE,
    MAKEFILE_TEMPLATE,
    VSCODE_TASKS_TEMPLATE,
    VSCODE_LAUNCH_TEMPLATE,
)

logger = logging.getLogger("algodeck.workspace")

class WorkspaceBuilder:
    def __init__(self, base_dir: Path = settings.workspace_dir):
        self.base_dir = base_dir
        self.base_dir.mkdir(parents=True, exist_ok=True)

    def create_problem_workspace(self, analysis: Dict[str, Any], original_pdf: Path = None) -> Path:
        """
        Builds complete competitive programming workspace for a given problem:
        - Creates folder <workspace>/<problem_id>/
        - Generates Olympic C++ template
        - Generates Brute force template & random test generator
        - Writes all test cases to tests/ (in and out files)
        - Configures Makefile & VS Code settings
        - Copies problem statement PDF
        - Automatically launches VS Code
        """
        problem_id = analysis.get("problem_id", "zad").lower()
        title = analysis.get("title", problem_id)
        time_limit = analysis.get("time_limit_sec", 1.0)
        memory_limit = analysis.get("memory_limit_mb", 256)

        problem_dir = self.base_dir / problem_id
        problem_dir.mkdir(parents=True, exist_ok=True)

        tests_dir = problem_dir / "tests"
        tests_dir.mkdir(parents=True, exist_ok=True)

        vscode_dir = problem_dir / ".vscode"
        vscode_dir.mkdir(parents=True, exist_ok=True)

        # 1. Main C++ source file
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

        # 2. Brute-force template (for stress-testing)
        brute_cpp = problem_dir / "brute.cpp"
        if not brute_cpp.exists():
            brute_cpp.write_text(
                BRUTE_CPP_TEMPLATE.format(
                    title=title,
                    problem_id=problem_id
                ),
                encoding="utf-8"
            )

        # 3. Test generator script
        gen_py = problem_dir / "gen.py"
        if not gen_py.exists():
            gen_py.write_text(
                GEN_PY_TEMPLATE.format(
                    title=title,
                    problem_id=problem_id
                ),
                encoding="utf-8"
            )
            # Make executable
            gen_py.chmod(0o755)

        # 4. Makefile
        makefile = problem_dir / "Makefile"
        makefile.write_text(
            MAKEFILE_TEMPLATE.format(problem_id=problem_id),
            encoding="utf-8"
        )

        # 5. VS Code configurations
        tasks_json = vscode_dir / "tasks.json"
        tasks_json.write_text(
            VSCODE_TASKS_TEMPLATE.format(problem_id=problem_id),
            encoding="utf-8"
        )

        launch_json = vscode_dir / "launch.json"
        launch_json.write_text(
            VSCODE_LAUNCH_TEMPLATE.format(problem_id=problem_id),
            encoding="utf-8"
        )

        # 6. Save tests to files
        tests: List[Dict[str, Any]] = analysis.get("tests", [])
        saved_tests = []
        for i, test in enumerate(tests, 1):
            prefix = "edge" if test.get("is_edge_case") else "test"
            in_filename = f"{prefix}_{i}.in"
            out_filename = f"{prefix}_{i}.out"

            in_path = tests_dir / in_filename
            out_path = tests_dir / out_filename

            in_path.write_text(test.get("input", "").strip() + "\n", encoding="utf-8")
            out_path.write_text(test.get("expected_output", "").strip() + "\n", encoding="utf-8")

            saved_tests.append({
                "id": f"{prefix}_{i}",
                "name": test.get("name", f"Test {i}"),
                "in_file": in_filename,
                "out_file": out_filename,
                "is_edge_case": test.get("is_edge_case", False),
                "description": test.get("description", "")
            })

        # 7. Copy PDF if provided
        if original_pdf and original_pdf.exists():
            shutil.copy2(original_pdf, problem_dir / "statement.pdf")

        # 8. problem.json metadata manifest
        problem_manifest = {
            "problem_id": problem_id,
            "title": title,
            "time_limit_sec": time_limit,
            "memory_limit_mb": memory_limit,
            "summary": analysis.get("summary", ""),
            "input_format": analysis.get("input_format", ""),
            "output_format": analysis.get("output_format", ""),
            "constraints": analysis.get("constraints", ""),
            "recommended_approach": analysis.get("recommended_approach", ""),
            "tests": saved_tests,
            "workspace_path": str(problem_dir.resolve()),
            "source_file": str(main_cpp.resolve())
        }
        (problem_dir / "problem.json").write_text(
            json.dumps(problem_manifest, indent=2, ensure_ascii=False),
            encoding="utf-8"
        )

        # 9. Auto-launch VS Code
        if settings.auto_launch_vscode:
            self.open_in_vscode(problem_dir, main_cpp)

        logger.info(f"Problem workspace generated at {problem_dir}")
        return problem_dir

    def open_in_vscode(self, problem_dir: Path, source_file: Path):
        """Launches VS Code with the workspace and source file open."""
        try:
            # Check if 'code' command is in PATH
            if shutil.which("code"):
                subprocess.Popen(
                    ["code", str(problem_dir), str(source_file)],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    start_new_session=True
                )
                logger.info("Launched Visual Studio Code.")
            else:
                logger.warning("'code' executable not found in PATH. Skipping auto-launch.")
        except Exception as e:
            logger.warning(f"Failed to auto-launch VS Code: {e}")
