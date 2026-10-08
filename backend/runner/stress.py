import logging
import subprocess
import time
from pathlib import Path
from typing import Dict, Any, Optional

from backend.config import settings

logger = logging.getLogger("algodeck.stress")

class StressTester:
    def __init__(self, workspace_dir: Path = settings.workspace_dir):
        self.workspace_dir = workspace_dir
        self.is_running = False

    def run_stress(self, problem_id: str, max_iterations: int = 100) -> Dict[str, Any]:
        """
        Runs automated stress testing between solution.cpp and brute.cpp using gen.py.
        Stops on first discrepancy or crash and saves the counterexample.
        """
        pdir = self.workspace_dir / problem_id.lower()
        sol_src = pdir / f"{problem_id}.cpp"
        brute_src = pdir / "brute.cpp"
        gen_src = pdir / "gen.py"

        if not brute_src.exists() or not gen_src.exists():
            return {
                "success": False,
                "error": "Brak pliku brute.cpp lub gen.py w katalogu zadania."
            }

        # 1. Compile solution
        sol_bin = pdir / problem_id
        cmd_sol = ["g++", "-O3", "-std=c++20", str(sol_src.name), "-o", str(sol_bin.name)]
        proc = subprocess.run(cmd_sol, cwd=str(pdir), capture_output=True, text=True)
        if proc.returncode != 0:
            return {"success": False, "error": f"Błąd kompilacji rozwiązania:\n{proc.stderr}"}

        # 2. Compile brute
        brute_bin = pdir / "brute"
        cmd_brute = ["g++", "-O3", "-std=c++20", str(brute_src.name), "-o", str(brute_bin.name)]
        proc_brute = subprocess.run(cmd_brute, cwd=str(pdir), capture_output=True, text=True)
        if proc_brute.returncode != 0:
            return {"success": False, "error": f"Błąd kompilacji brute force:\n{proc_brute.stderr}"}

        self.is_running = True
        logger.info(f"Rozpoczęto stress-testing dla {problem_id} (do {max_iterations} iteracji)...")

        for iteration in range(1, max_iterations + 1):
            if not self.is_running:
                return {"success": True, "stopped": True, "iterations": iteration - 1, "status": "Zatrzymano przez użytkownika"}

            # A. Generate test
            gen_proc = subprocess.run(["python3", str(gen_src.name)], cwd=str(pdir), capture_output=True, text=True)
            test_input = gen_proc.stdout

            # B. Run solution
            try:
                sol_proc = subprocess.run([f"./{problem_id}"], cwd=str(pdir), input=test_input, capture_output=True, text=True, timeout=2.0)
            except subprocess.TimeoutExpired:
                self._save_counterexample(pdir, test_input, "TIMEOUT", "TLE")
                return {
                    "success": False,
                    "iteration": iteration,
                    "verdict": "TLE",
                    "input": test_input,
                    "details": "Rozwiązanie przekroczyło limit czasu na wygenerowanym teście."
                }

            if sol_proc.returncode != 0:
                self._save_counterexample(pdir, test_input, sol_proc.stderr, "RTE")
                return {
                    "success": False,
                    "iteration": iteration,
                    "verdict": "RTE",
                    "input": test_input,
                    "details": f"Rozwiązanie zakończyło się błędem wykonania (kod {sol_proc.returncode}):\n{sol_proc.stderr}"
                }

            # C. Run brute
            brute_proc = subprocess.run(["./brute"], cwd=str(pdir), input=test_input, capture_output=True, text=True, timeout=5.0)

            out_sol = sol_proc.stdout.strip()
            out_brute = brute_proc.stdout.strip()

            if out_sol != out_brute:
                self._save_counterexample(pdir, test_input, out_brute, "WA")
                return {
                    "success": False,
                    "iteration": iteration,
                    "verdict": "WA",
                    "input": test_input,
                    "expected": out_brute,
                    "actual": out_sol,
                    "details": f"Znaleziono rozbieżność w iteracji #{iteration}!"
                }

        self.is_running = False
        return {
            "success": True,
            "iterations": max_iterations,
            "status": f"Wszystkie {max_iterations} losowych testów zakończone sukcesem (zgodne z brute)!"
        }

    def stop(self):
        self.is_running = False

    def _save_counterexample(self, pdir: Path, test_input: str, expected_out: str, tag: str):
        (pdir / "tests" / "stress_fail.in").write_text(test_input, encoding="utf-8")
        (pdir / "tests" / "stress_fail.out").write_text(expected_out, encoding="utf-8")
