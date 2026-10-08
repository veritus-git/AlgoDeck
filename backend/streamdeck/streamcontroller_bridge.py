import json
import logging
import os
import shutil
import subprocess
from pathlib import Path
from typing import Dict, Any, List

from backend.config import settings

logger = logging.getLogger("algodeck.streamcontroller")

class StreamControllerBridge:
    def __init__(self, workspace_dir: Path = settings.workspace_dir):
        self.workspace_dir = workspace_dir

    def generate_page_for_problem(self, problem_id: str, analysis: Dict[str, Any]) -> Dict[str, Any]:
        """
        Generates StreamController compatible page definitions and helper shell scripts
        under <workspace>/<problem_id>/scripts/ so buttons can be triggered directly or via REST.
        """
        pdir = self.workspace_dir / problem_id.lower()
        scripts_dir = pdir / "streamdeck_scripts"
        scripts_dir.mkdir(parents=True, exist_ok=True)

        # 1. Generate standalone shell scripts for each action
        actions = {
            "0_build_fast.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/0",
            "1_build_debug.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/1",
            "2_run_all.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/2",
            "3_benchmark.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/3",
            "4_kill.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/4",
            "5_test_1.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/5",
            "6_test_2.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/6",
            "7_test_3.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/7",
            "8_copy_in_1.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/8",
            "9_copy_in_2.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/9",
            "10_edge_1.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/10",
            "11_edge_2.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/11",
            "12_stress.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/12",
            "13_vscode.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/13",
            "14_next_task.sh": f"curl -s -X POST http://127.0.0.1:8080/api/streamdeck/press/14"
        }

        for fname, cmd in actions.items():
            script_path = scripts_dir / fname
            script_path.write_text(f"#!/usr/bin/env bash\n{cmd}\n", encoding="utf-8")
            script_path.chmod(0o755)

        # 2. Generate StreamController page manifest
        page_config = {
            "name": f"AlgoDeck: {problem_id.upper()}",
            "description": f"Automated profile for Olympic problem {analysis.get('title', problem_id)}",
            "grid": {"rows": 3, "cols": 5},
            "keys": [
                {"index": 0, "name": "Build (O3)", "script": str((scripts_dir / "0_build_fast.sh").resolve())},
                {"index": 1, "name": "Debug (ASan)", "script": str((scripts_dir / "1_build_debug.sh").resolve())},
                {"index": 2, "name": "Run All Tests", "script": str((scripts_dir / "2_run_all.sh").resolve())},
                {"index": 3, "name": "Benchmark", "script": str((scripts_dir / "3_benchmark.sh").resolve())},
                {"index": 4, "name": "Emergency Stop", "script": str((scripts_dir / "4_kill.sh").resolve())},
                {"index": 5, "name": "Test 1", "script": str((scripts_dir / "5_test_1.sh").resolve())},
                {"index": 6, "name": "Test 2", "script": str((scripts_dir / "6_test_2.sh").resolve())},
                {"index": 7, "name": "Test 3", "script": str((scripts_dir / "7_test_3.sh").resolve())},
                {"index": 8, "name": "Copy Input 1", "script": str((scripts_dir / "8_copy_in_1.sh").resolve())},
                {"index": 9, "name": "Copy Input 2", "script": str((scripts_dir / "9_copy_in_2.sh").resolve())},
                {"index": 10, "name": "Edge Case 1", "script": str((scripts_dir / "10_edge_1.sh").resolve())},
                {"index": 11, "name": "Edge Case 2", "script": str((scripts_dir / "11_edge_2.sh").resolve())},
                {"index": 12, "name": "Stress Testing", "script": str((scripts_dir / "12_stress.sh").resolve())},
                {"index": 13, "name": "Open in VS Code", "script": str((scripts_dir / "13_vscode.sh").resolve())},
                {"index": 14, "name": "Next Problem", "script": str((scripts_dir / "14_next_task.sh").resolve())}
            ]
        }

        page_file = pdir / f"{problem_id}_streamcontroller.json"
        page_file.write_text(json.dumps(page_config, indent=2, ensure_ascii=False), encoding="utf-8")

        # 3. Try switching page via StreamController CLI if installed
        self.try_switch_streamcontroller_page(problem_id)

        return page_config

    def try_switch_streamcontroller_page(self, problem_id: str):
        """Attempts to switch StreamController active page using flatpak CLI."""
        if not shutil.which("flatpak"):
            return
        try:
            subprocess.run(
                ["flatpak", "run", "com.core447.StreamController", "--change-page", "default", problem_id.upper()],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=2
            )
        except Exception as e:
            logger.debug(f"StreamController page switch notice: {e}")
