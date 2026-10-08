import json
import logging
import os
import subprocess
from pathlib import Path
from typing import Dict, Any

from backend.config import settings

logger = logging.getLogger("algodeck.streamcontroller")

STREAMCONTROLLER_DATA = Path(os.path.expanduser("~/.var/app/com.core447.StreamController/data"))
STREAMCONTROLLER_PAGES = STREAMCONTROLLER_DATA / "pages"
STREAMCONTROLLER_ICONS = STREAMCONTROLLER_DATA / "custom_icons"

class StreamControllerBridge:
    def __init__(self, workspace_dir: Path = settings.workspace_dir):
        self.workspace_dir = workspace_dir

    def ensure_vector_icons(self):
        """Upewnia się, że ikony w stylu Dev & Secrets istnieją."""
        STREAMCONTROLLER_ICONS.mkdir(parents=True, exist_ok=True)
        test_icon = STREAMCONTROLLER_ICONS / "icon_test_sec.png"
        task_icon = STREAMCONTROLLER_ICONS / "icon_task_sec.png"
        if not test_icon.exists() or not task_icon.exists():
            try:
                from backend.streamdeck.icons_generator import generate_icons
                generate_icons()
            except Exception as e:
                logger.warning(f"Nie udało się wygenerować ikon: {e}")

    def generate_page_for_problem(self, problem_id: str, analysis: Dict[str, Any]) -> bool:
        """
        Zapisuje stronę do bazy StreamControllera z 5 przyciskami NA ŚRODKU (rząd 1)
        oraz płynną nawigacją między projektami ze stylem Dev & Secrets.
        """
        prob_id = problem_id.upper()
        pdir = (self.workspace_dir / problem_id.lower()).resolve()

        STREAMCONTROLLER_PAGES.mkdir(parents=True, exist_ok=True)
        self.ensure_vector_icons()

        # Ikony w stylu Dev & Secrets
        icon_test = STREAMCONTROLLER_ICONS / "icon_test_sec.png"
        icon_play = STREAMCONTROLLER_ICONS / "icon_play_sec.png"
        icon_kill = STREAMCONTROLLER_ICONS / "icon_kill_sec.png"
        icon_vscode = STREAMCONTROLLER_ICONS / "icon_vscode_sec.png"
        icon_task = STREAMCONTROLLER_ICONS / "icon_task_sec.png"
        icon_next = STREAMCONTROLLER_ICONS / "icon_next_sec.png"
        icon_prev = STREAMCONTROLLER_ICONS / "icon_prev_sec.png"

        test_cmd = f'bash "{pdir}/.algo/test.sh"'
        run_cmd = f'x-terminal-emulator -e bash -c \'"{pdir}/.algo/run.sh"; echo ""; read -p "Zakończono. Naciśnij Enter..."\' 2>/dev/null || bash "{pdir}/.algo/run.sh"'
        kill_cmd = f'bash "{pdir}/.algo/kill.sh"'
        vscode_cmd = f'$HOME/.local/bin/sd_algo_code.sh "{pdir}" "{pdir}/{problem_id}.cpp"'
        cycle_cmd = f'$HOME/.local/bin/sd_algo_switch.sh next "{problem_id}"'
        next_cmd = f'$HOME/.local/bin/sd_algo_switch.sh next "{problem_id}"'
        prev_cmd = f'$HOME/.local/bin/sd_algo_switch.sh prev "{problem_id}"'

        def make_key(cmd: str, icon_path: Path):
            return {
                "states": {
                    "0": {
                        "actions": [
                            {
                                "id": "com_core447_OSPlugin::EasyCommand",
                                "settings": {
                                    "command": cmd
                                }
                            }
                        ],
                        "labels": {},
                        "background": {
                            "color": [16, 18, 27, 255]
                        },
                        "media": {
                            "path": str(icon_path.resolve()),
                            "size": 1.0,
                            "valign": 0.0,
                            "halign": 0.0,
                            "fill-mode": "cover"
                        },
                        "image-control-action": 0,
                        "label-control-actions": [0, 0, 0],
                        "background-control-action": 0
                    }
                }
            }

        page_data = {
            "screensaver": {},
            "keys": {
                # Rząd 0 (Górny) - Płynna nawigacja między zapisanymi zadaniami
                "0x0": make_key(prev_cmd, icon_prev),  # ⏮️ Poprzednie zadanie
                "4x0": make_key(next_cmd, icon_next),  # ⏭️ Następne zadanie

                # Rząd 1 (ŚRODEK) - 5 GŁÓWNYCH PRZYCISKÓW NA ŚRODKU!
                "0x1": make_key(test_cmd, icon_test),      # 🚀 TESTUJ
                "1x1": make_key(run_cmd, icon_play),       # ▶️ ODPAL
                "2x1": make_key(kill_cmd, icon_kill),      # 🛑 KILL
                "3x1": make_key(vscode_cmd, icon_vscode),  # 💻 VS CODE (FULL EKRAN)
                "4x1": make_key(cycle_cmd, icon_task),     # 🔄 ZADANIE (KOLEJNE)
            }
        }

        # Zapisz stronę do bazy StreamControllera
        target_page_path = STREAMCONTROLLER_PAGES / f"{prob_id}.json"
        target_page_path.write_text(json.dumps(page_data, indent=4), encoding="utf-8")
        logger.info(f"Utworzono stronę StreamControllera: {target_page_path}")

        # Natychmiast przełącz na tę stronę
        self.switch_to_page(prob_id)
        return True

    def switch_to_page(self, page_name: str):
        """Przełącza aktywną stronę na Stream Decku."""
        serial = "A00SA6042JGA63"
        try:
            subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", "/com/core447/StreamController",
                "--method", "com.core447.StreamController.ChangePage",
                serial, page_name
            ], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            logger.info(f"Przełączono stronę StreamControllera na {page_name} przez DBus.")
        except Exception:
            pass

        sd_page_script = Path(os.path.expanduser("~/.local/bin/sd_page.sh"))
        if sd_page_script.exists():
            try:
                subprocess.run([str(sd_page_script), page_name], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            except Exception:
                pass
