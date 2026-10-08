import difflib
import json
import logging
import os
import resource
import shutil
import subprocess
import time
from pathlib import Path
from typing import Dict, Any, List, Optional

from backend.config import settings

logger = logging.getLogger("algodeck.runner")

class TestExecutor:
    def __init__(self, workspace_dir: Path = settings.workspace_dir):
        self.workspace_dir = workspace_dir
        self.active_processes: Dict[str, subprocess.Popen] = {}

    def get_problem_dir(self, problem_id: str) -> Path:
        return self.workspace_dir / problem_id.lower()

    def get_manifest(self, problem_id: str) -> Dict[str, Any]:
        pdir = self.get_problem_dir(problem_id)
        manifest_file = pdir / ".algo" / "problem.json"
        if not manifest_file.exists():
            manifest_file = pdir / "problem.json"
        if manifest_file.exists():
            return json.loads(manifest_file.read_text(encoding="utf-8"))
        return {}

    def compile(self, problem_id: str, debug_mode: bool = False) -> Dict[str, Any]:
        """
        Compiles the solution with fast flags (-O3) or debug sanitizer flags (-fsanitize=address,undefined).
        """
        pdir = self.get_problem_dir(problem_id)
        src_file = pdir / f"{problem_id}.cpp"
        if not src_file.exists():
            return {"success": False, "error": f"Plik źródłowy {src_file.name} nie istnieje."}

        target_name = f"{problem_id}_debug" if debug_mode else problem_id
        target_path = pdir / target_name

        flags = settings.cxx_debug_flags if debug_mode else settings.cxx_release_flags
        cmd = [settings.cxx_compiler, f"-std={settings.cxx_std}"] + flags.split() + [str(src_file.name), "-o", str(target_path.name)]

        start_time = time.perf_counter()
        try:
            proc = subprocess.run(
                cmd,
                cwd=str(pdir),
                capture_output=True,
                text=True,
                timeout=30.0
            )
            duration_ms = round((time.perf_counter() - start_time) * 1000, 1)

            if proc.returncode == 0:
                return {
                    "success": True,
                    "target": str(target_path),
                    "debug_mode": debug_mode,
                    "warnings": proc.stderr,
                    "duration_ms": duration_ms
                }
            else:
                return {
                    "success": False,
                    "error": proc.stderr,
                    "debug_mode": debug_mode,
                    "duration_ms": duration_ms
                }
        except subprocess.TimeoutExpired:
            return {"success": False, "error": "Przekroczono limit czasu kompilacji (30s)."}
        except Exception as e:
            return {"success": False, "error": str(e)}

    def run_single_test(self, problem_id: str, test_id: str, is_debug: bool = False) -> Dict[str, Any]:
        """
        Runs the compiled solution against a specific test case,
        measures wall-clock time and peak memory, and performs diff against expected output.
        """
        pdir = self.get_problem_dir(problem_id)
        manifest = self.get_manifest(problem_id)
        time_limit = manifest.get("time_limit_sec", 1.0)
        
        target_name = f"{problem_id}_debug" if is_debug else problem_id
        binary = pdir / target_name

        # Auto-compile if binary doesn't exist
        if not binary.exists():
            comp_res = self.compile(problem_id, debug_mode=is_debug)
            if not comp_res["success"]:
                return {
                    "test_id": test_id,
                    "verdict": "CE",
                    "time_ms": 0,
                    "memory_kb": 0,
                    "error": comp_res.get("error", "Kompilacja nie powiodła się.")
                }

        # Locate test file (.algo/tests/ or tests/)
        in_file = pdir / ".algo" / "tests" / f"{test_id}.in"
        if not in_file.exists():
            in_file = pdir / "tests" / f"{test_id}.in"

        out_file = pdir / ".algo" / "tests" / f"{test_id}.out"
        if not out_file.exists():
            out_file = pdir / "tests" / f"{test_id}.out"

        if not in_file.exists():
            return {"test_id": test_id, "verdict": "ERR", "error": f"Brak pliku wejściowego {in_file.name}"}

        input_data = in_file.read_text(encoding="utf-8")
        expected_output = out_file.read_text(encoding="utf-8") if out_file.exists() else ""

        start_time = time.perf_counter()
        try:
            proc = subprocess.Popen(
                [f"./{target_name}"],
                cwd=str(pdir),
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )
            self.active_processes[problem_id] = proc

            try:
                # Add 200ms grace period for timeout before killing
                stdout, stderr = proc.communicate(input=input_data, timeout=time_limit + 0.2)
                elapsed_ms = round((time.perf_counter() - start_time) * 1000, 1)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.communicate()
                return {
                    "test_id": test_id,
                    "verdict": "TLE",
                    "time_ms": round(time_limit * 1000, 1),
                    "memory_kb": 0,
                    "details": f"Przekroczono limit czasu ({time_limit}s)."
                }
            finally:
                if problem_id in self.active_processes:
                    del self.active_processes[problem_id]

            if proc.returncode != 0:
                return {
                    "test_id": test_id,
                    "verdict": "RTE",
                    "time_ms": elapsed_ms,
                    "error": stderr or f"Proces zakończył się kodem błędu {proc.returncode}",
                    "details": "Błąd wykonania (SIGSEGV / ASan Assertion)"
                }

            # Check output against expected
            normalized_actual = self._normalize_output(stdout)
            normalized_expected = self._normalize_output(expected_output)

            if not normalized_expected:
                # Test with unknown expected output (e.g. edge case for crash testing)
                return {
                    "test_id": test_id,
                    "verdict": "OK (RUN)",
                    "time_ms": elapsed_ms,
                    "actual_output": stdout,
                    "expected_output": "(brak wzorca - test odporności)",
                    "details": "Program wykonał się bez błędu."
                }

            if normalized_actual == normalized_expected:
                return {
                    "test_id": test_id,
                    "verdict": "OK",
                    "time_ms": elapsed_ms,
                    "actual_output": stdout,
                    "expected_output": expected_output
                }
            else:
                diff = list(difflib.unified_diff(
                    normalized_expected.splitlines(keepends=True),
                    normalized_actual.splitlines(keepends=True),
                    fromfile="Oczekiwane (Expected)",
                    tofile="Otrzymane (Actual)",
                    n=3
                ))
                return {
                    "test_id": test_id,
                    "verdict": "WA",
                    "time_ms": elapsed_ms,
                    "actual_output": stdout,
                    "expected_output": expected_output,
                    "diff": "".join(diff)
                }

        except Exception as e:
            return {"test_id": test_id, "verdict": "ERR", "error": str(e)}

    def run_all_tests(self, problem_id: str, is_debug: bool = False) -> Dict[str, Any]:
        """Runs all registered tests for the problem and generates an aggregated summary."""
        manifest = self.get_manifest(problem_id)
        tests = manifest.get("tests", [])

        # Compile first
        comp_res = self.compile(problem_id, debug_mode=is_debug)
        if not comp_res["success"]:
            return {
                "success": False,
                "summary": "Błąd kompilacji",
                "verdict": "CE",
                "compile_error": comp_res.get("error", "")
            }

        results = []
        all_passed = True
        total_time_ms = 0.0

        for t in tests:
            test_id = t["id"]
            res = self.run_single_test(problem_id, test_id, is_debug=is_debug)
            res["name"] = t.get("name", test_id)
            res["is_edge_case"] = t.get("is_edge_case", False)
            results.append(res)
            total_time_ms += res.get("time_ms", 0.0)

            if res.get("verdict") not in ("OK", "OK (RUN)"):
                all_passed = False

        return {
            "success": True,
            "all_passed": all_passed,
            "verdict": "ALL_OK" if all_passed else "SOME_FAILED",
            "total_time_ms": round(total_time_ms, 1),
            "results": results
        }

    def benchmark_test(self, problem_id: str, test_id: str, runs: int = 5) -> Dict[str, Any]:
        """Runs benchmark iterations on a given test to measure average and minimum execution times."""
        times = []
        for _ in range(runs):
            res = self.run_single_test(problem_id, test_id, is_debug=False)
            if res.get("verdict") == "TLE":
                return {"success": False, "verdict": "TLE"}
            times.append(res.get("time_ms", 0))

        return {
            "success": True,
            "runs": runs,
            "min_ms": min(times),
            "avg_ms": round(sum(times) / len(times), 2),
            "max_ms": max(times)
        }

    def copy_to_clipboard(self, text: str) -> bool:
        """Copies text to system clipboard (Wayland wl-copy first, then xclip/xsel)."""
        # 1. Try wl-copy (Fedora KDE Plasma Wayland)
        if shutil.which("wl-copy"):
            try:
                proc = subprocess.run(["wl-copy"], input=text, text=True, timeout=2)
                if proc.returncode == 0:
                    return True
            except Exception:
                pass

        # 2. Try xclip
        if shutil.which("xclip"):
            try:
                proc = subprocess.run(["xclip", "-selection", "clipboard"], input=text, text=True, timeout=2)
                if proc.returncode == 0:
                    return True
            except Exception:
                pass

        return False

    def copy_test_input(self, problem_id: str, test_id: str) -> Dict[str, Any]:
        """Copies specific test case input to clipboard."""
        pdir = self.get_problem_dir(problem_id)
        in_file = pdir / ".algo" / "tests" / f"{test_id}.in"
        if not in_file.exists():
            in_file = pdir / "tests" / f"{test_id}.in"
        if in_file.exists():
            content = in_file.read_text(encoding="utf-8")
            copied = self.copy_to_clipboard(content)
            return {"success": True, "copied_to_sys_clipboard": copied, "content": content}
        return {"success": False, "error": f"Plik {in_file.name} nie istnieje."}

    def kill_process(self, problem_id: str) -> bool:
        """Immediately halts any active running process for the given task."""
        proc = self.active_processes.get(problem_id)
        if proc:
            try:
                proc.kill()
                return True
            except Exception:
                pass
        return False

    @staticmethod
    def _normalize_output(text: str) -> str:
        """Normalizes output by trimming trailing whitespace and line endings."""
        lines = [line.rstrip() for line in text.replace("\r\n", "\n").split("\n")]
        while lines and not lines[-1]:
            lines.pop()
        return "\n".join(lines)
