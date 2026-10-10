import json
import logging
import os
import subprocess
from pathlib import Path
from typing import Dict, Any, List, Optional

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
        """Upewnia się, że geometryczne ikony na czarnym tle istnieją."""
        STREAMCONTROLLER_ICONS.mkdir(parents=True, exist_ok=True)
        from backend.streamdeck.icons_generator import generate_all_base_icons
        generate_all_base_icons()

    def sync_page_to_streamcontroller(self, page_name: str, page_data: Dict[str, Any]):
        """
        Zapisuje stronę na dysk ORAZ przeładowuje definicję w StreamControllerze bez usuwania strony (brak migania do Main!).
        """
        prob_id = page_name.upper()
        json_str = json.dumps(page_data, indent=4)
        target_page_path = STREAMCONTROLLER_PAGES / f"{prob_id}.json"

        # Zapisz natychmiast na dysk
        try:
            target_page_path.write_text(json_str, encoding="utf-8")
        except Exception as e:
            logger.error(f"Nie udało się zapisać pliku {target_page_path}: {e}")

        # Załaduj nową definicję do pamięci StreamControllera przez DBus bez RemovePage
        gvariant_str = json.dumps(json_str)
        try:
            subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", "/com/core447/StreamController",
                "--method", "com.core447.StreamController.AddPage",
                prob_id, gvariant_str
            ], timeout=2, capture_output=True, text=True)
        except Exception as e:
            logger.warning(f"Błąd DBus AddPage: {e}")

    def generate_page_for_problem(self, problem_id: str, analysis: Optional[Dict[str, Any]] = None, switch_now: bool = True) -> bool:
        """
        Zapisuje stronę zadania w StreamControllerze:
        - Rząd 0 (GÓRA): Środkowe 3 klawisze (1x0, 2x0, 3x0) to DUŻE POJEDYNCZE LITERY zadania (np. [C] [H] [W]).
        - Rząd 1 (ŚRODEK): [</> VS CODE] [✓ TESTUJ] [▶ ODPAL] [⏹ KILL] [⊞ GALERIA]
        - Rząd 2 (DÓŁ): [‹ POPRZEDNIE]                        [› NASTĘPNE]
        Wszystkie klawisze mają czyste czarne tło i brak nakładanych napisów (labels: {}).
        """
        prob_id = problem_id.upper()
        pdir = (self.workspace_dir / problem_id.lower()).resolve()
        analysis = analysis or {}

        STREAMCONTROLLER_PAGES.mkdir(parents=True, exist_ok=True)
        self.ensure_vector_icons()

        from backend.streamdeck.icons_generator import generate_letter_icon

        # Ikony akcji na czarnym tle
        icon_plus = STREAMCONTROLLER_ICONS / "icon_plus.png"
        icon_test = STREAMCONTROLLER_ICONS / "icon_test_sec.png"
        icon_play = STREAMCONTROLLER_ICONS / "icon_play_sec.png"
        icon_kill = STREAMCONTROLLER_ICONS / "icon_kill_sec.png"
        icon_task = STREAMCONTROLLER_ICONS / "icon_task_sec.png"
        icon_equals = STREAMCONTROLLER_ICONS / "icon_equals.png"

        # Komendy
        new_task_cmd = '$HOME/.local/bin/sd_algo_new_task.sh'
        test_cmd = f'gnome-terminal --title="AlgoDeck Testy - {prob_id}" -- bash "{pdir}/.algo/test.sh" 2>/dev/null || x-terminal-emulator -e bash "{pdir}/.algo/test.sh"'
        run_cmd = f'$HOME/.local/bin/sd_algo_run_vscode.sh "{pdir}" "{problem_id}"'
        kill_cmd = f'bash "{pdir}/.algo/kill.sh"'
        menu_cmd = f'$HOME/.local/bin/sd_algo_switch.sh menu "{problem_id}"'
        prev_cmd = f'$HOME/.local/bin/sd_algo_switch.sh prev "{problem_id}"'
        next_cmd = f'$HOME/.local/bin/sd_algo_switch.sh next "{problem_id}"'

        # Sprawdź liczbę zadań: jeśli 1 lub 0, strzałki nawigacji są wyszarzone i nieaktywne
        projects = []
        if self.workspace_dir.exists():
            for p in self.workspace_dir.iterdir():
                if p.is_dir() and not p.name.startswith(".") and p.name.lower() != "tests":
                    projects.append(p.name.lower())
        num_projects = len(projects)

        if num_projects <= 1:
            icon_left = STREAMCONTROLLER_ICONS / "icon_arrow_left_disabled.png"
            icon_right = STREAMCONTROLLER_ICONS / "icon_arrow_right_disabled.png"
            left_cmd = "true"
            right_cmd = "true"
        else:
            icon_left = STREAMCONTROLLER_ICONS / "icon_arrow_left.png"
            icon_right = STREAMCONTROLLER_ICONS / "icon_arrow_right.png"
            left_cmd = prev_cmd
            right_cmd = next_cmd

        def make_key(cmd: str, icon_path: Optional[Path] = None):
            state_data: Dict[str, Any] = {
                "actions": [
                    {
                        "id": "com_core447_OSPlugin::EasyCommand",
                        "settings": {
                            "command": cmd
                        }
                    }
                ],
                "labels": {},  # Puste labels - zero nakładających się napisów!
                "background": {
                    "color": [0, 0, 0, 255]
                },
                "image-control-action": 0,
                "label-control-actions": [0, 0, 0],
                "background-control-action": 0
            }
            if icon_path and icon_path.exists():
                state_data["media"] = {
                    "path": str(icon_path.resolve()),
                    "size": 1.0,
                    "valign": 0.0,
                    "halign": 0.0,
                    "fill-mode": "cover"
                }
            return {
                "states": {
                    "0": state_data
                }
            }

        def make_empty_key():
            return {
                "states": {
                    "0": {
                        "actions": [],
                        "labels": {},
                        "background": {
                            "color": [0, 0, 0, 255]
                        },
                        "image-control-action": 0,
                        "label-control-actions": [0, 0, 0],
                        "background-control-action": 0
                    }
                }
            }

        # 1. RZĄD 0: Przycisk 1 i 5 to '=', a środkowe 3 klawisze to litery zadania
        letters = list(prob_id[:3].ljust(3))
        icon_l1 = generate_letter_icon(letters[0])
        icon_l2 = generate_letter_icon(letters[1])
        icon_l3 = generate_letter_icon(letters[2])

        info_cmd = f'notify-send "AlgoDeck" "Zadanie: {analysis.get("title", prob_id)} [{prob_id}]" 2>/dev/null || true'

        page_data = {
            "screensaver": {},
            "keys": {
                # RZĄD 0 (GÓRA): [ = ] [ L1 ] [ L2 ] [ L3 ] [ = ]
                "0x0": make_key(info_cmd, icon_equals),
                "1x0": make_key(info_cmd, icon_l1),
                "2x0": make_key(info_cmd, icon_l2),
                "3x0": make_key(info_cmd, icon_l3),
                "4x0": make_key(info_cmd, icon_equals),

                # RZĄD 1 (ŚRODEK): [ + ] [ ✓ ] [ ▶ ] [ ⏹ ] [ ⊞ ]
                "0x1": make_key(new_task_cmd, icon_plus),
                "1x1": make_key(test_cmd, icon_test),
                "2x1": make_key(run_cmd, icon_play),
                "3x1": make_key(kill_cmd, icon_kill),
                "4x1": make_key(menu_cmd, icon_task),

                # RZĄD 2 (DÓŁ): [ ‹ ] [   ] [   ] [   ] [ › ] (znaki '=' usunięte z dolnego rzędu)
                "0x2": make_key(left_cmd, icon_left),
                "1x2": make_empty_key(),
                "2x2": make_empty_key(),
                "3x2": make_empty_key(),
                "4x2": make_key(right_cmd, icon_right),
            }
        }

        self.sync_page_to_streamcontroller(prob_id, page_data)
        self.generate_menu_page(active_id=problem_id.lower())
        if switch_now:
            self.switch_to_page(prob_id)
        return True

    def remove_problem(self, problem_id: str):
        """Usuwa stronę zadania ze StreamControllera i odświeża menu."""
        prob_id = problem_id.upper()
        target_page_path = STREAMCONTROLLER_PAGES / f"{prob_id}.json"
        try:
            subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", "/com/core447/StreamController",
                "--method", "com.core447.StreamController.RemovePage",
                prob_id
            ], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except Exception:
            pass
        target_page_path.unlink(missing_ok=True)
        (STREAMCONTROLLER_ICONS / f"task_{problem_id.lower()}.png").unlink(missing_ok=True)
        self.sync_all_problems()

    def sync_all_problems(self):
        """Automatycznie generuje i synchronizuje czyste profile dla wszystkich zadań w workspace."""
        self.ensure_vector_icons()
        if not self.workspace_dir.exists():
            return
        
        for p in sorted(self.workspace_dir.iterdir()):
            if not p.is_dir() or p.name.startswith("."):
                continue
            pid = p.name.lower()
            if pid in ("tests",):
                continue
            
            mfile = p / ".algo" / "problem.json"
            if not mfile.exists():
                mfile = p / "problem.json"
            
            analysis = {"problem_id": pid, "title": pid.upper()}
            if mfile.exists():
                try:
                    analysis = json.loads(mfile.read_text(encoding="utf-8"))
                except Exception:
                    pass
            
            self.generate_page_for_problem(pid, analysis, switch_now=False)
        
        self.generate_menu_page()

    def generate_menu_page(self, active_id: str = ""):
        """
        Generuje galerię zadań (ALGO_MENU):
        - Zadania zajmują górne 10 przycisków (Rząd 0: 5 zadań, Rząd 1: 5 zadań).
        - Dolny lewy przycisk (0x2) to powrót ← do aktywnego zadania.
        - Jeśli zadań jest > 10, w prawym dolnym rogu (4x2) pojawia się strzałka › do kolejnej strony.
        - Grafiki samych zadań zachowane w oryginalnym, dopracowanym stylu.
        """
        from backend.streamdeck.icons_generator import generate_task_button_icon

        icon_back = STREAMCONTROLLER_ICONS / "icon_arrow_back.png"
        icon_next = STREAMCONTROLLER_ICONS / "icon_arrow_right.png"
        icon_prev = STREAMCONTROLLER_ICONS / "icon_arrow_left.png"

        # Zbierz wszystkie zadania
        projects: List[Dict[str, str]] = []
        if self.workspace_dir.exists():
            for p in sorted(self.workspace_dir.iterdir()):
                if not p.is_dir() or p.name.startswith("."):
                    continue
                pid = p.name.lower()
                if pid in ("tests",):
                    continue
                
                title = pid.upper()
                mfile = p / ".algo" / "problem.json"
                if not mfile.exists():
                    mfile = p / "problem.json"
                if mfile.exists():
                    try:
                        m = json.loads(mfile.read_text(encoding="utf-8"))
                        title = m.get("title", title)
                    except Exception:
                        pass
                projects.append({"id": pid, "title": title})

        # Pozycje dla 10 zadań (2 górne rzędy)
        slot_positions = [
            "0x0", "1x0", "2x0", "3x0", "4x0",
            "0x1", "1x1", "2x1", "3x1", "4x1"
        ]

        # Podział na strony (po 10 zadań)
        chunk_size = 10
        total_pages = max(1, (len(projects) + chunk_size - 1) // chunk_size)

        for page_idx in range(total_pages):
            page_name = "ALGO_MENU" if page_idx == 0 else f"ALGO_MENU_{page_idx + 1}"
            start_i = page_idx * chunk_size
            page_projects = projects[start_i : start_i + chunk_size]

            keys: Dict[str, Any] = {}

            # 1. Dodaj zadania w 2 górnych rzędach
            for idx, proj in enumerate(page_projects):
                pos = slot_positions[idx]
                pid = proj["id"]
                title = proj["title"]
                is_active = (pid == active_id.lower())

                icon_path = generate_task_button_icon(pid, title, is_active=is_active)
                cmd = f'$HOME/.local/bin/sd_algo_switch.sh to "{pid}"'

                keys[pos] = {
                    "states": {
                        "0": {
                            "actions": [{"id": "com_core447_OSPlugin::EasyCommand", "settings": {"command": cmd}}],
                            "labels": {},
                            "background": {"color": [0, 0, 0, 255]},
                            "media": {
                                "path": str(icon_path.resolve()),
                                "size": 1.0, "valign": 0.0, "halign": 0.0, "fill-mode": "cover"
                            },
                            "image-control-action": 0, "label-control-actions": [0, 0, 0], "background-control-action": 0
                        }
                    }
                }

            # 2. Przycisk powrotu w lewym dolnym rogu (0x2)
            if page_idx == 0:
                back_cmd = "$HOME/.local/bin/sd_algo_menu_back.sh"
                back_icon = icon_back
            else:
                prev_target = "ALGO_MENU" if page_idx == 1 else f"ALGO_MENU_{page_idx}"
                back_cmd = f'gdbus call --session --dest com.core447.StreamController --object-path /com/core447/StreamController --method com.core447.StreamController.ChangePage "{SERIAL}" "{prev_target}" >/dev/null 2>&1 || true'
                back_icon = icon_prev

            keys["0x2"] = {
                "states": {
                    "0": {
                        "actions": [{"id": "com_core447_OSPlugin::EasyCommand", "settings": {"command": back_cmd}}],
                        "labels": {},
                        "background": {"color": [0, 0, 0, 255]},
                        "media": {
                            "path": str(back_icon.resolve()),
                            "size": 1.0, "valign": 0.0, "halign": 0.0, "fill-mode": "cover"
                        },
                        "image-control-action": 0, "label-control-actions": [0, 0, 0], "background-control-action": 0
                    }
                }
            }

            # 3. Jeśli są kolejne zadania, w prawym dolnym rogu (4x2) pojawia się strzałka w prawo ›
            if page_idx < total_pages - 1:
                next_target = f"ALGO_MENU_{page_idx + 2}"
                next_cmd = f'gdbus call --session --dest com.core447.StreamController --object-path /com/core447/StreamController --method com.core447.StreamController.ChangePage "{SERIAL}" "{next_target}" >/dev/null 2>&1 || true'
                keys["4x2"] = {
                    "states": {
                        "0": {
                            "actions": [{"id": "com_core447_OSPlugin::EasyCommand", "settings": {"command": next_cmd}}],
                            "labels": {},
                            "background": {"color": [0, 0, 0, 255]},
                            "media": {
                                "path": str(icon_next.resolve()),
                                "size": 1.0, "valign": 0.0, "halign": 0.0, "fill-mode": "cover"
                            },
                            "image-control-action": 0, "label-control-actions": [0, 0, 0], "background-control-action": 0
                        }
                    }
                }

            self.sync_page_to_streamcontroller(page_name, {"screensaver": {}, "keys": keys})

    def switch_to_page(self, page_name: str) -> bool:
        """Wysyła sygnał DBus do StreamControllera, by natychmiast wyświetlić stronę na fizycznym Stream Decku."""
        prob_id = page_name.upper()
        try:
            res = subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", "/com/core447/StreamController",
                "--method", "com.core447.StreamController.ChangePage",
                SERIAL, prob_id
            ], timeout=2, capture_output=True, text=True)
            return res.returncode == 0
        except Exception:
            return False
