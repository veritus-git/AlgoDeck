import json
import logging
import os
import subprocess
from pathlib import Path
from typing import Dict, Any, List

from backend.config import settings

logger = logging.getLogger("algodeck.streamcontroller")

STREAMCONTROLLER_DATA = Path(os.path.expanduser("~/.var/app/com.core447.StreamController/data"))
STREAMCONTROLLER_PAGES = STREAMCONTROLLER_DATA / "pages"
STREAMCONTROLLER_ICONS = STREAMCONTROLLER_DATA / "custom_icons"
SERIAL = "A00SA6042JGA63"

class StreamControllerBridge:
    def __init__(self, workspace_dir: Path = settings.workspace_dir):
        self.workspace_dir = workspace_dir

    def ensure_vector_icons(self):
        """Upewnia się, że ikony w stylu Dev & Secrets istnieją."""
        STREAMCONTROLLER_ICONS.mkdir(parents=True, exist_ok=True)
        test_icon = STREAMCONTROLLER_ICONS / "icon_test_sec.png"
        arrow_left = STREAMCONTROLLER_ICONS / "icon_arrow_left.png"
        if not test_icon.exists() or not arrow_left.exists():
            try:
                from backend.streamdeck.icons_generator import generate_icons
                generate_icons()
            except Exception as e:
                logger.warning(f"Nie udało się wygenerować ikon bazowych: {e}")

    def sync_page_to_streamcontroller(self, page_name: str, page_data: Dict[str, Any]):
        """
        Zapisuje stronę na dysk ORAZ przeładowuje pamięć podręczną StreamControllera przez DBus.
        """
        import time
        prob_id = page_name.upper()
        json_str = json.dumps(page_data, indent=4)
        target_page_path = STREAMCONTROLLER_PAGES / f"{prob_id}.json"

        # 1. Usuń istniejącą wersję strony z pamięci podręcznej StreamControllera
        try:
            subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", "/com/core447/StreamController",
                "--method", "com.core447.StreamController.RemovePage",
                prob_id
            ], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            time.sleep(0.05)
        except Exception:
            pass

        # 2. Załaduj zaktualizowany JSON do StreamControllera przez AddPage (co zapisze stronę i zaktualizuje UI)
        gvariant_str = json.dumps(json_str)
        added_ok = False
        try:
            res = subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", "/com/core447/StreamController",
                "--method", "com.core447.StreamController.AddPage",
                prob_id, gvariant_str
            ], timeout=2, capture_output=True, text=True)
            if res.returncode == 0:
                added_ok = True
                logger.info(f"Pomyślnie załadowano stronę {prob_id} do StreamControllera.")
            else:
                logger.warning(f"Błąd AddPage DBus: {res.stderr}")
        except Exception as e:
            logger.warning(f"Wyjątek AddPage DBus: {e}")

        # Jeśli DBus AddPage nie zapisał pliku, zapisujemy bezpośrednio na dysku jako gwarancję
        if not target_page_path.exists() or not added_ok:
            try:
                target_page_path.write_text(json_str, encoding="utf-8")
            except Exception as e:
                logger.error(f"Nie udało się zapisać pliku {target_page_path}: {e}")

    def generate_page_for_problem(self, problem_id: str, analysis: Dict[str, Any]) -> bool:
        """
        Zapisuje stronę do StreamControllera z układem:
        - Rząd 1 (ŚRODEK): [VS CODE] [TESTUJ] [ODPAL] [KILL] [ZADANIA]
        - Rząd 2 (DÓŁ): [< (POPRZ)]                      [> (NAST)]
        """
        prob_id = problem_id.upper()
        pdir = (self.workspace_dir / problem_id.lower()).resolve()

        STREAMCONTROLLER_PAGES.mkdir(parents=True, exist_ok=True)
        self.ensure_vector_icons()

        # Ikony
        icon_vscode = STREAMCONTROLLER_ICONS / "icon_vscode_sec.png"
        icon_test = STREAMCONTROLLER_ICONS / "icon_test_sec.png"
        icon_play = STREAMCONTROLLER_ICONS / "icon_play_sec.png"
        icon_kill = STREAMCONTROLLER_ICONS / "icon_kill_sec.png"
        icon_task = STREAMCONTROLLER_ICONS / "icon_task_sec.png"
        icon_arrow_left = STREAMCONTROLLER_ICONS / "icon_arrow_left.png"
        icon_arrow_right = STREAMCONTROLLER_ICONS / "icon_arrow_right.png"

        # Komendy
        vscode_cmd = f'$HOME/.local/bin/sd_algo_code.sh "{pdir}" "{pdir}/{problem_id}.cpp"'
        # TESTUJ: systemowy gnome-terminal z pełnym podglądem wejścia/wyjścia/diffów
        test_cmd = f'gnome-terminal --title="AlgoDeck Testy - {prob_id}" -- bash "{pdir}/.algo/test.sh" 2>/dev/null || x-terminal-emulator -e bash "{pdir}/.algo/test.sh"'
        # ODPAL: wbudowany terminal VS Code
        run_cmd = f'$HOME/.local/bin/sd_algo_run_vscode.sh "{pdir}" "{problem_id}"'
        kill_cmd = f'bash "{pdir}/.algo/kill.sh"'
        # ZADANIA: otwiera SUBMENU z listą zadań
        menu_cmd = f'$HOME/.local/bin/sd_algo_switch.sh menu "{problem_id}"'
        # Nawigacja na dole: < i >
        prev_cmd = f'$HOME/.local/bin/sd_algo_switch.sh prev "{problem_id}"'
        next_cmd = f'$HOME/.local/bin/sd_algo_switch.sh next "{problem_id}"'

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

        # RZĄD 0 (GÓRA): Wskaźnik nazwy aktywnego zadania
        title = analysis.get("title") or prob_id
        from backend.streamdeck.icons_generator import generate_active_task_badge
        icon_badge = generate_active_task_badge(problem_id, title)
        badge_cmd = f'notify-send "AlgoDeck" "Zadanie: {title} ({prob_id})" 2>/dev/null || true'

        page_data = {
            "screensaver": {},
            "keys": {
                # RZĄD 0 (GÓRA):
                # 2x0: Nazwa aktualnego zadania (widoczna na górnej linii)
                "2x0": make_key(badge_cmd, icon_badge),

                # RZĄD 1 (ŚRODEK):
                # 0x1: VS CODE (na maksa z lewej)
                # 1x1: TESTUJ (po lewej od środka)
                # 2x1: ODPAL (NA ŚRODKU)
                # 3x1: KILL (po prawej od środka)
                # 4x1: ZADANIA (na maksa z prawej - otwiera submenu)
                "0x1": make_key(vscode_cmd, icon_vscode),
                "1x1": make_key(test_cmd, icon_test),
                "2x1": make_key(run_cmd, icon_play),
                "3x1": make_key(kill_cmd, icon_kill),
                "4x1": make_key(menu_cmd, icon_task),

                # RZĄD 2 (DÓŁ): Nawigacja estetycznymi strzałkami bez tekstu
                # 0x2: < (Poprzednie zadanie)
                # 4x2: > (Następne zadanie)
                "0x2": make_key(prev_cmd, icon_arrow_left),
                "4x2": make_key(next_cmd, icon_arrow_right),
            }
        }

        # Zsynchronizuj stronę z bazą i pamięcią StreamControllera
        self.sync_page_to_streamcontroller(prob_id, page_data)

        # Zaktualizuj także stronę submenu z wszystkimi zadaniami
        self.generate_menu_page()

        # Natychmiast przełącz na tę stronę
        self.switch_to_page(prob_id)
        return True

    def generate_menu_page(self):
        """
        Generuje stronę ALGO_MENU (Submenu ze wszystkimi zadaniami).
        Każde zadanie ma pełną nazwę i skalowaną czcionkę.
        Zawiera także strzałkę powrotu ← do aktywnego zadania.
        """
        from backend.streamdeck.icons_generator import generate_task_button_icon

        keys: Dict[str, Any] = {}
        icon_back = STREAMCONTROLLER_ICONS / "icon_arrow_back.png"

        # 1. Przycisk powrotu do aktywnego zadania (Rząd 2, Lewy dół: 0x2)
        keys["0x2"] = {
            "states": {
                "0": {
                    "actions": [
                        {
                            "id": "com_core447_OSPlugin::EasyCommand",
                            "settings": {
                                "command": "$HOME/.local/bin/sd_algo_menu_back.sh"
                            }
                        }
                    ],
                    "labels": {},
                    "background": {"color": [16, 18, 27, 255]},
                    "media": {
                        "path": str(icon_back.resolve()),
                        "size": 1.0, "valign": 0.0, "halign": 0.0, "fill-mode": "cover"
                    },
                    "image-control-action": 0, "label-control-actions": [0, 0, 0], "background-control-action": 0
                }
            }
        }

        # 2. Zbierz wszystkie zapisane zadania
        projects: List[Dict[str, str]] = []
        if self.workspace_dir.exists():
            for p in sorted(self.workspace_dir.iterdir()):
                if not p.is_dir():
                    continue
                pid = p.name.lower()
                if pid == "tests":
                    continue
                
                # Odczytaj tytuł z manifestu
                title = pid.upper()
                mfile = p / ".algo" / "problem.json"
                if not mfile.exists():
                    mfile = p / "problem.json"
                if mfile.exists():
                    try:
                        mdata = json.loads(mfile.read_text(encoding="utf-8"))
                        title = mdata.get("title") or pid.upper()
                    except Exception:
                        pass
                
                projects.append({"id": pid, "title": title})

        # 3. Rozmieść zadania na siatce (wiersz 0 i 1)
        grid_positions = [
            "0x0", "1x0", "2x0", "3x0", "4x0",
            "0x1", "1x1", "2x1", "3x1", "4x1"
        ]

        for i, proj in enumerate(projects[:len(grid_positions)]):
            pos = grid_positions[i]
            pid = proj["id"]
            title = proj["title"]

            # Wygeneruj ikonę z pełną nazwą
            task_icon_path = generate_task_button_icon(pid, title)
            switch_cmd = f'$HOME/.local/bin/sd_algo_switch.sh to "{pid}"'

            keys[pos] = {
                "states": {
                    "0": {
                        "actions": [
                            {
                                "id": "com_core447_OSPlugin::EasyCommand",
                                "settings": {
                                    "command": switch_cmd
                                }
                            }
                        ],
                        "labels": {},
                        "background": {"color": [16, 18, 27, 255]},
                        "media": {
                            "path": str(task_icon_path.resolve()),
                            "size": 1.0, "valign": 0.0, "halign": 0.0, "fill-mode": "cover"
                        },
                        "image-control-action": 0, "label-control-actions": [0, 0, 0], "background-control-action": 0
                    }
                }
            }

        menu_page_data = {
            "screensaver": {},
            "keys": keys
        }

        self.sync_page_to_streamcontroller("ALGO_MENU", menu_page_data)

    def switch_to_page(self, page_name: str):
        """Przełącza aktywną stronę na Stream Decku."""
        page_upper = page_name.upper()

        # Zapisz w pliku pomocniczym
        try:
            Path("/tmp/algodeck_active_task.txt").write_text(page_upper, encoding="utf-8")
        except Exception:
            pass

        # 1. Przez ChangePage na głównym obiekcie (najbardziej niezawodne)
        try:
            subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", "/com/core447/StreamController",
                "--method", "com.core447.StreamController.ChangePage",
                SERIAL, page_upper
            ], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            logger.info(f"ChangePage na {page_upper} przez DBus.")
        except Exception:
            pass

        # 2. Przez DBus na dedykowanym kontrolerze
        try:
            subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", f"/com/core447/StreamController/controllers/{SERIAL}",
                "--method", "com.core447.StreamController.Controller.SetActivePage",
                page_upper
            ], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            logger.info(f"SetActivePage na {page_upper} przez DBus kontrolera.")
        except Exception:
            pass
