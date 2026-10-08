import json
import logging
import os
import subprocess
from pathlib import Path
from typing import Dict, Any

from backend.config import settings
from backend.streamdeck.renderer import ButtonRenderer

logger = logging.getLogger("algodeck.streamcontroller")

STREAMCONTROLLER_DATA = Path(os.path.expanduser("~/.var/app/com.core447.StreamController/data"))
STREAMCONTROLLER_PAGES = STREAMCONTROLLER_DATA / "pages"
STREAMCONTROLLER_ICONS = STREAMCONTROLLER_DATA / "custom_icons"

class StreamControllerBridge:
    def __init__(self, workspace_dir: Path = settings.workspace_dir):
        self.workspace_dir = workspace_dir

    def generate_page_for_problem(self, problem_id: str, analysis: Dict[str, Any]) -> bool:
        """
        Zapisuje stronę bezpośrednio do bazy stron StreamControllera:
        ~/.var/app/com.core447.StreamController/data/pages/<problem_id>.json
        i wywołuje ~/.local/bin/sd_page.sh <problem_id> aby natychmiast przełączyć stronę!
        """
        prob_id = problem_id.upper()
        pdir = (self.workspace_dir / problem_id.lower()).resolve()

        STREAMCONTROLLER_PAGES.mkdir(parents=True, exist_ok=True)
        STREAMCONTROLLER_ICONS.mkdir(parents=True, exist_ok=True)

        # 1. Wygeneruj ikony LCD dla klawiszy
        icon_test = STREAMCONTROLLER_ICONS / f"algo_{problem_id}_test.png"
        icon_run = STREAMCONTROLLER_ICONS / f"algo_{problem_id}_run.png"
        icon_kill = STREAMCONTROLLER_ICONS / f"algo_{problem_id}_kill.png"
        icon_code = STREAMCONTROLLER_ICONS / f"algo_{problem_id}_code.png"
        icon_back = STREAMCONTROLLER_ICONS / f"algo_{problem_id}_back.png"

        ButtonRenderer.render_key(label="TESTUJ", icon="🚀", subtext="ALL", status="OK").save(icon_test)
        ButtonRenderer.render_key(label="ODPAL", icon="▶️", subtext="RUN", status="DEBUG").save(icon_run)
        ButtonRenderer.render_key(label="KILL", icon="🛑", subtext="STOP", status="DANGER").save(icon_kill)
        ButtonRenderer.render_key(label="VSCODE", icon="💻", subtext="OPEN", status="ACTION").save(icon_code)
        ButtonRenderer.render_key(label="POWRÓT", icon="🔙", subtext="MAIN", status="IDLE").save(icon_back)

        # 2. Utwórz konfigurację strony StreamControllera
        test_cmd = f'bash "{pdir}/.algo/test.sh"'
        run_cmd = f'x-terminal-emulator -e bash -c \'"{pdir}/.algo/run.sh"; echo ""; read -p "Zakończono. Naciśnij Enter..."\' 2>/dev/null || bash "{pdir}/.algo/run.sh"'
        kill_cmd = f'bash "{pdir}/.algo/kill.sh"'
        vscode_cmd = f'code "{pdir}" "{pdir}/{problem_id}.cpp"'
        back_cmd = f'$HOME/.local/bin/sd_page.sh Main'

        def make_key(cmd: str, icon_path: Path, bg_color: list):
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
                            "color": bg_color
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
                # Wiersz 0: Tylko najważniejsze przyciski!
                "0x0": make_key(test_cmd, icon_test, [6, 78, 59, 255]),     # TESTUJ
                "1x0": make_key(run_cmd, icon_run, [23, 37, 84, 255]),      # ODPAL
                "2x0": make_key(kill_cmd, icon_kill, [69, 10, 10, 255]),    # KILL
                "3x0": make_key(vscode_cmd, icon_code, [30, 27, 75, 255]),  # VS CODE
                "4x0": make_key(back_cmd, icon_back, [18, 22, 32, 255])     # POWRÓT DO MAIN
            }
        }

        # Zapisz stronę do bazy StreamControllera
        target_page_path = STREAMCONTROLLER_PAGES / f"{prob_id}.json"
        target_page_path.write_text(json.dumps(page_data, indent=4), encoding="utf-8")
        logger.info(f"Utworzono stronę StreamControllera: {target_page_path}")

        # 3. Natychmiast przełącz stronę w StreamControllerze!
        self.switch_to_page(prob_id)
        return True

    def switch_to_page(self, page_name: str):
        """Przełącza aktywną stronę na Stream Decku."""
        # 1. Przez DBus bezpośrednio
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

        # 2. Przez sd_page.sh
        sd_page_script = Path(os.path.expanduser("~/.local/bin/sd_page.sh"))
        if sd_page_script.exists():
            try:
                subprocess.run([str(sd_page_script), page_name], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            except Exception:
                pass
