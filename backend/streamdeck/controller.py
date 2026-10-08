import asyncio
import logging
from typing import Dict, Any, List, Optional, Callable

from backend.config import settings
from backend.runner.executor import TestExecutor
from backend.runner.stress import StressTester

logger = logging.getLogger("algodeck.streamdeck")

class StreamDeckController:
    def __init__(self, executor: TestExecutor):
        self.executor = executor
        self.stress_tester = StressTester()
        self.active_problem_id: Optional[str] = None
        self.known_problems: List[str] = []
        
        # State of each of the 15 keys
        self.keys_state: List[Dict[str, Any]] = [
            {"id": i, "label": "", "icon": "", "subtext": "", "status": "IDLE", "action": ""}
            for i in range(15)
        ]

        # Listener callbacks (e.g. for WebSockets or hardware)
        self.listeners: List[Callable[[List[Dict[str, Any]]], None]] = []

    def set_active_problem(self, problem_id: str):
        """Sets active problem and configures all 15 buttons accordingly."""
        self.active_problem_id = problem_id.lower()
        if self.active_problem_id not in self.known_problems:
            self.known_problems.append(self.active_problem_id)

        manifest = self.executor.get_manifest(self.active_problem_id)
        tests = manifest.get("tests", [])

        # Find official tests and edge cases
        official_tests = [t for t in tests if not t.get("is_edge_case", False)]
        edge_tests = [t for t in tests if t.get("is_edge_case", False)]

        # Row 1 (Index 0-4): Main Commands
        self._set_key(0, label="BUILD", icon="🔨", subtext="O3", status="IDLE", action="compile_fast")
        self._set_key(1, label="DEBUG", icon="🐞", subtext="ASan", status="DEBUG", action="compile_debug")
        self._set_key(2, label="ALL", icon="🚀", subtext="TESTS", status="ACTION", action="run_all")
        self._set_key(3, label="BENCH", icon="⏱️", subtext="SPEED", status="IDLE", action="bench_test_1")
        self._set_key(4, label="KILL", icon="🛑", subtext="STOP", status="DANGER", action="kill_process")

        # Row 2 (Index 5-9): Official Tests & Copying
        # Key 5: Test 1
        t1_id = official_tests[0]["id"] if len(official_tests) > 0 else "test_1"
        self._set_key(5, label="TEST 1", icon="🧪", subtext="RUN", status="IDLE", action=f"run_test:{t1_id}")

        # Key 6: Test 2
        t2_id = official_tests[1]["id"] if len(official_tests) > 1 else "test_2"
        self._set_key(6, label="TEST 2", icon="🧪", subtext="RUN", status="IDLE", action=f"run_test:{t2_id}")

        # Key 7: Test 3
        t3_id = official_tests[2]["id"] if len(official_tests) > 2 else "test_3"
        self._set_key(7, label="TEST 3", icon="🧪", subtext="RUN", status="IDLE", action=f"run_test:{t3_id}")

        # Key 8: Copy In 1
        self._set_key(8, label="COPY 1", icon="📋", subtext="CLIP", status="IDLE", action=f"copy_in:{t1_id}")

        # Key 9: Copy In 2
        self._set_key(9, label="COPY 2", icon="📋", subtext="CLIP", status="IDLE", action=f"copy_in:{t2_id}")

        # Row 3 (Index 10-14): Edge cases, Stress & Navigation
        # Key 10: Edge 1
        e1_id = edge_tests[0]["id"] if len(edge_tests) > 0 else "edge_1"
        self._set_key(10, label="EDGE 1", icon="⚠️", subtext="CORNER", status="IDLE", action=f"run_test:{e1_id}")

        # Key 11: Edge 2
        e2_id = edge_tests[1]["id"] if len(edge_tests) > 1 else "edge_2"
        self._set_key(11, label="EDGE 2", icon="⚠️", subtext="CORNER", status="IDLE", action=f"run_test:{e2_id}")

        # Key 12: Stress Test
        self._set_key(12, label="STRESS", icon="⚔️", subtext="BRUTE", status="IDLE", action="toggle_stress")

        # Key 13: Open VS Code
        self._set_key(13, label="VSCODE", icon="💻", subtext="OPEN", status="ACTION", action="open_vscode")

        # Key 14: Switch Task
        self._set_key(14, label=self.active_problem_id.upper(), icon="🔄", subtext="NEXT", status="IDLE", action="cycle_task")

        self._notify_listeners()

    def _set_key(self, index: int, label: str, icon: str, subtext: str, status: str, action: str):
        if 0 <= index < 15:
            self.keys_state[index] = {
                "id": index,
                "label": label,
                "icon": icon,
                "subtext": subtext,
                "status": status,
                "action": action
            }

    def update_key_status(self, index: int, status: str, subtext: Optional[str] = None):
        if 0 <= index < 15:
            self.keys_state[index]["status"] = status
            if subtext is not None:
                self.keys_state[index]["subtext"] = subtext
            self._notify_listeners()

    async def execute_key(self, key_index: int) -> Dict[str, Any]:
        """Executes the action mapped to the given key index (0-14)."""
        if not (0 <= key_index < 15) or not self.active_problem_id:
            return {"success": False, "error": "Brak aktywnego zadania lub nieprawidłowy klawisz."}

        key = self.keys_state[key_index]
        action = key.get("action", "")
        problem_id = self.active_problem_id

        # Visual feedback: set key to RUNNING
        old_status = key["status"]
        old_subtext = key["subtext"]
        self.update_key_status(key_index, "RUNNING", "RUN...")

        result = {"success": True}

        try:
            if action == "compile_fast":
                res = self.executor.compile(problem_id, debug_mode=False)
                if res["success"]:
                    self.update_key_status(key_index, "OK", f"{res['duration_ms']}ms")
                else:
                    self.update_key_status(key_index, "CE", "ERROR")
                result = res

            elif action == "compile_debug":
                res = self.executor.compile(problem_id, debug_mode=True)
                if res["success"]:
                    self.update_key_status(key_index, "OK", "ASan OK")
                else:
                    self.update_key_status(key_index, "CE", "ERROR")
                result = res

            elif action == "run_all":
                res = self.executor.run_all_tests(problem_id)
                if res.get("all_passed"):
                    self.update_key_status(key_index, "OK", f"{res['total_time_ms']}ms")
                else:
                    self.update_key_status(key_index, "WA", "FAILED")
                # Also update individual test keys
                self._sync_test_keys_from_run_all(res)
                result = res

            elif action.startswith("run_test:"):
                test_id = action.split(":")[1]
                res = self.executor.run_single_test(problem_id, test_id)
                verdict = res.get("verdict", "ERR")
                time_ms = res.get("time_ms", 0)
                
                status_map = {
                    "OK": "OK",
                    "OK (RUN)": "OK",
                    "WA": "WA",
                    "TLE": "TLE",
                    "RTE": "RTE",
                    "CE": "CE"
                }
                new_status = status_map.get(verdict, "WA")
                subtext = f"{time_ms}ms" if verdict in ("OK", "OK (RUN)") else verdict
                self.update_key_status(key_index, new_status, subtext)
                result = res

            elif action.startswith("copy_in:"):
                test_id = action.split(":")[1]
                res = self.executor.copy_test_input(problem_id, test_id)
                self.update_key_status(key_index, "COPIED", "COPIED!")
                # Reset after 1.5 seconds
                asyncio.create_task(self._reset_key_after_delay(key_index, "IDLE", "CLIP", 1.5))
                result = res

            elif action == "bench_test_1":
                manifest = self.executor.get_manifest(problem_id)
                tests = manifest.get("tests", [])
                t_id = tests[0]["id"] if tests else "test_1"
                res = self.executor.benchmark_test(problem_id, t_id)
                if res["success"]:
                    self.update_key_status(key_index, "OK", f"{res['avg_ms']}ms")
                else:
                    self.update_key_status(key_index, "TLE", "TLE")
                result = res

            elif action == "kill_process":
                stopped = self.executor.kill_process(problem_id)
                self.stress_tester.stop()
                self.update_key_status(key_index, "DANGER", "KILLED")
                asyncio.create_task(self._reset_key_after_delay(key_index, "DANGER", "STOP", 1.5))
                result = {"success": True, "stopped": stopped}

            elif action == "toggle_stress":
                res = self.stress_tester.run_stress(problem_id, max_iterations=50)
                if res.get("success"):
                    self.update_key_status(key_index, "OK", "PASS 50")
                else:
                    self.update_key_status(key_index, "WA", res.get("verdict", "FAIL"))
                result = res

            elif action == "open_vscode":
                pdir = self.executor.get_problem_dir(problem_id)
                src = pdir / f"{problem_id}.cpp"
                from backend.workspace.builder import WorkspaceBuilder
                WorkspaceBuilder().open_in_vscode(pdir, src)
                self.update_key_status(key_index, "OK", "OPENED")
                asyncio.create_task(self._reset_key_after_delay(key_index, "ACTION", "OPEN", 2.0))
                result = {"success": True}

            elif action == "cycle_task":
                self._cycle_next_task()
                result = {"success": True, "active": self.active_problem_id}

        except Exception as e:
            logger.error(f"Error executing key {key_index} ({action}): {e}")
            self.update_key_status(key_index, "WA", "ERR")
            result = {"success": False, "error": str(e)}

        return result

    def _sync_test_keys_from_run_all(self, run_all_res: Dict[str, Any]):
        """Updates individual test buttons when batch 'Run All' finishes."""
        results = run_all_res.get("results", [])
        for r in results:
            t_id = r.get("test_id")
            verdict = r.get("verdict", "ERR")
            time_ms = r.get("time_ms", 0)
            status = "OK" if verdict in ("OK", "OK (RUN)") else ("WA" if verdict == "WA" else "TLE")
            subtext = f"{time_ms}ms" if status == "OK" else verdict

            # Find matching key
            for idx, k in enumerate(self.keys_state):
                if k.get("action") == f"run_test:{t_id}":
                    self.update_key_status(idx, status, subtext)

    def _cycle_next_task(self):
        """Cycles to the next known problem workspace."""
        if not self.known_problems:
            return
        try:
            curr_idx = self.known_problems.index(self.active_problem_id)
            next_idx = (curr_idx + 1) % len(self.known_problems)
            self.set_active_problem(self.known_problems[next_idx])
        except ValueError:
            self.set_active_problem(self.known_problems[0])

    async def _reset_key_after_delay(self, index: int, status: str, subtext: str, delay: float):
        await asyncio.sleep(delay)
        self.update_key_status(index, status, subtext)

    def add_listener(self, callback: Callable[[List[Dict[str, Any]]], None]):
        self.listeners.append(callback)

    def _notify_listeners(self):
        for listener in self.listeners:
            try:
                listener(self.keys_state)
            except Exception as e:
                logger.error(f"Error in StreamDeck listener callback: {e}")
