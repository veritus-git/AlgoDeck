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

    def get_ordered_problem_ids(self) -> List[str]:
        """
        Zwraca listę identyfikatorów zadań posortowanych ściśle chronologicznie
        (od najstarszego do najnowszego - pierwsze utworzone zadanie ma indeks 1).
        """
        if not self.workspace_dir.exists():
            return []

        entries = []
        for p in self.workspace_dir.iterdir():
            if not p.is_dir() or p.name.startswith("."):
                continue
            pid = p.name.lower()
            if pid in ("tests",):
                continue
            if not ((p / f"{pid}.cpp").exists() or (p / ".algo").exists()):
                continue

            created_at = None
            mfile = p / ".algo" / "problem.json"
            if not mfile.exists():
                mfile = p / "problem.json"
            if mfile.exists():
                try:
                    m = json.loads(mfile.read_text(encoding="utf-8"))
                    if "created_at" in m and isinstance(m["created_at"], (int, float)):
                        created_at = float(m["created_at"])
                except Exception:
                    pass

            if created_at is None:
                try:
                    st = p.stat()
                    created_at = min(st.st_mtime, st.st_ctime)
                except Exception:
                    created_at = 0.0

            entries.append((created_at, pid))

        # Sortuj rosnąco według daty utworzenia: najstarsze zadanie = indeks 0 (zadanie #1)
        entries.sort(key=lambda x: (x[0], x[1]))
        return [pid for _, pid in entries]

    def ensure_os_plugin(self) -> bool:
        """
        Upewnia się, że oficjalny plugin com_core447_OSPlugin jest zainstalowany w StreamControllerze.
        Bez niego na przyciskach pojawia się ostrzegawcza kropka, a akcje 'EasyCommand' nie działają.
        """
        plugin_dir = Path.home() / ".var/app/com.core447.StreamController/data/plugins/com_core447_OSPlugin"
        manifest_file = plugin_dir / "manifest.json"
        if manifest_file.exists():
            return True

        logger.info("Instalacja brakującego pluginu com_core447_OSPlugin w StreamControllerze...")
        plugin_dir.mkdir(parents=True, exist_ok=True)

        # 1. Kopiuj z lokalnych zasobów offline repozytorium
        local_resource = Path(__file__).resolve().parent / "resources" / "com_core447_OSPlugin"
        installed = False
        if local_resource.exists() and (local_resource / "manifest.json").exists():
            try:
                for item in local_resource.glob("*"):
                    dest = plugin_dir / item.name
                    if item.is_dir():
                        shutil.copytree(item, dest, dirs_exist_ok=True)
                    else:
                        shutil.copy2(item, dest)
                installed = True
                logger.info("Skopiowano com_core447_OSPlugin z lokalnych zasobów AlgoDeck.")
            except Exception as e:
                logger.warning(f"Błąd kopiowania lokalnego com_core447_OSPlugin: {e}")

        # 2. Pobierz z sieci jeśli lokalna kopia nie była dostępna
        if not installed or not manifest_file.exists():
            try:
                import urllib.request
                import tarfile
                import io
                url = "https://github.com/StreamController/OSPlugin/archive/refs/heads/main.tar.gz"
                req = urllib.request.Request(url, headers={"User-Agent": "AlgoDeck"})
                with urllib.request.urlopen(req, timeout=10) as resp:
                    tar_bytes = resp.read()
                with tarfile.open(fileobj=io.BytesIO(tar_bytes), mode="r:gz") as tar:
                    members = []
                    for m in tar.getmembers():
                        parts = Path(m.name).parts
                        if len(parts) > 1:
                            m.name = str(Path(*parts[1:]))
                            members.append(m)
                    tar.extractall(path=plugin_dir, members=members)
                installed = True
                logger.info("Pobrano com_core447_OSPlugin z oficjalnego repozytorium GitHub.")
            except Exception as e:
                logger.error(f"Nie udało się pobrać com_core447_OSPlugin: {e}")

        if installed and manifest_file.exists():
            try:
                subprocess.run(["flatpak", "kill", "com.core447.StreamController"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            except Exception:
                pass
            return True
        return False

    def ensure_vector_icons(self):
        """Upewnia się, że geometryczne ikony na czarnym tle oraz plugin OS istnieją."""
        self.ensure_os_plugin()
        STREAMCONTROLLER_ICONS.mkdir(parents=True, exist_ok=True)
        try:
            from backend.streamdeck.icons_generator import generate_all_base_icons
            generate_all_base_icons()
        except Exception as e:
            logger.warning(f"Błąd podczas sprawdzania ikon: {e}")

    def sync_page_to_streamcontroller(self, page_name: str, page_data: Dict[str, Any]):
        """
        Zapisuje stronę na dysk w katalogu stron StreamControllera.
        StreamController automatycznie monitoruje ten katalog i ładuje definicję w locie.
        """
        prob_id = page_name.upper()
        json_str = json.dumps(page_data, indent=4)
        target_page_path = STREAMCONTROLLER_PAGES / f"{prob_id}.json"

        try:
            # 1. Usuń poprzednią wersję ze StreamControllera (czyści pamięć i stary plik)
            subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", "/com/core447/StreamController",
                "--method", "com.core447.StreamController.RemovePage",
                prob_id
            ], timeout=1, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

            # 2. Dodaj zaktualizowaną stronę przez AddPage z poprawnym kodowaniem GVariant string
            gvariant_str = json.dumps(json_str)
            subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", "/com/core447/StreamController",
                "--method", "com.core447.StreamController.AddPage",
                prob_id, gvariant_str
            ], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

            # 3. Zawsze upewnij się, że plik na dysku zawiera kompletną definicję
            target_page_path.write_text(json_str, encoding="utf-8")
            logger.info(f"Pomyślnie zsynchronizowano stronę {prob_id} ze StreamControllerem.")
        except Exception as e:
            logger.error(f"Nie udało się zapisać pliku {target_page_path}: {e}")

    def generate_idle_page(self, switch_now: bool = False):
        """
        Generuje 'Ekran zachęty' (ALGO_IDLE) w stylu minimalistycznego menu głównego:
        - Rząd 0: [   ] [ ALGO ] [ ⚡ ] [ DECK ] [   ] (Logo jak w instalatorze)
        - Rząd 1: [   ] [      ] [ ▶ ZACZNIJ ] [      ] [   ] (Przycisk Start; 4x1 ZAWSZE pusty)
        - Rząd 2: [ ⚙ ] [      ] [     ] [      ] [   ] (Dyskretne Ustawienia)
        """
        self.ensure_vector_icons()

        icon_algo = STREAMCONTROLLER_ICONS / "idle_logo_algo.png"
        icon_bolt = STREAMCONTROLLER_ICONS / "idle_logo_bolt.png"
        icon_deck = STREAMCONTROLLER_ICONS / "idle_logo_deck.png"
        icon_start = STREAMCONTROLLER_ICONS / "idle_btn_start.png"
        icon_settings = STREAMCONTROLLER_ICONS / "icon_settings.png"

        start_cmd = '$HOME/.local/bin/sd_algo_panel.sh --tab=new'
        settings_cmd = '$HOME/.local/bin/sd_algo_panel.sh --tab=settings'

        def make_key(cmd: str, icon_path: Optional[Path] = None):
            state_data: Dict[str, Any] = {
                "actions": [{"id": "com_core447_OSPlugin::EasyCommand", "settings": {"command": cmd}}],
                "labels": {},
                "background": {"color": [0, 0, 0, 255]},
                "image-control-action": 0, "label-control-actions": [0, 0, 0], "background-control-action": 0
            }
            if icon_path and icon_path.exists():
                state_data["media"] = {
                    "path": str(icon_path.resolve()), "size": 1.0, "valign": 0.0, "halign": 0.0, "fill-mode": "cover"
                }
            return {"states": {"0": state_data}}

        def make_empty_key():
            return {
                "states": {"0": {"actions": [], "labels": {}, "background": {"color": [0, 0, 0, 255]},
                                 "image-control-action": 0, "label-control-actions": [0, 0, 0], "background-control-action": 0}}
            }

        page_data = {
            "screensaver": {},
            "keys": {
                # RZĄD 0: [   ] [ ALGO ] [ ⚡ ] [ DECK ] [   ]
                "0x0": make_empty_key(),
                "1x0": make_key(start_cmd, icon_algo),
                "2x0": make_key(start_cmd, icon_bolt),
                "3x0": make_key(start_cmd, icon_deck),
                "4x0": make_empty_key(),

                # RZĄD 1: [   ] [   ] [ ▶ ZACZNIJ ] [   ] [   ]
                "0x1": make_empty_key(),
                "1x1": make_empty_key(),
                "2x1": make_key(start_cmd, icon_start),
                "3x1": make_empty_key(),
                "4x1": make_empty_key(),

                # RZĄD 2: [ ⚙ ] [   ] [   ] [   ] [   ]
                "0x2": make_key(settings_cmd, icon_settings),
                "1x2": make_empty_key(),
                "2x2": make_empty_key(),
                "3x2": make_empty_key(),
                "4x2": make_empty_key(),
            }
        }
        self.sync_page_to_streamcontroller("ALGO_IDLE", page_data)
        if switch_now:
            self.switch_to_page("ALGO_IDLE")

    def generate_page_for_problem(self, problem_id: str, analysis: Optional[Dict[str, Any]] = None, switch_now: bool = True) -> bool:
        """
        Zapisuje stronę zadania w StreamControllerze:
        - Rząd 0: [ = ] [ L1 ] [ L2 ] [ L3 ] [ = ]
        - Rząd 1: [ + ] [ ✓ ] [ ▶ ] [ ⏹ ] [ ⊞ ]
        - Rząd 2: [ ⚙ ] [   ] [ ‹ ] [Nr ] [ › ] (Ustawienia, Pusty, Lewo, Nr zadania, Prawo)
        """
        prob_id = problem_id.upper()
        pdir = (self.workspace_dir / problem_id.lower()).resolve()
        analysis = analysis or {}

        STREAMCONTROLLER_PAGES.mkdir(parents=True, exist_ok=True)
        self.ensure_vector_icons()

        from backend.streamdeck.icons_generator import (
            generate_letter_icon, generate_settings_icon, generate_task_number_icon
        )

        # Pobierz wszystkie zadania w ścisłej kolejności chronologicznej utworzenia (najstarsze = 1/N)
        projects = self.get_ordered_problem_ids()
        total_projects = len(projects)
        current_idx = 1
        if problem_id.lower() in projects:
            current_idx = projects.index(problem_id.lower()) + 1

        # Ikony akcji na czarnym tle
        icon_plus = STREAMCONTROLLER_ICONS / "icon_plus.png"
        icon_test = STREAMCONTROLLER_ICONS / "icon_test_sec.png"
        icon_play = STREAMCONTROLLER_ICONS / "icon_play_sec.png"
        icon_kill = STREAMCONTROLLER_ICONS / "icon_kill_sec.png"
        icon_task = STREAMCONTROLLER_ICONS / "icon_task_sec.png"
        icon_equals = STREAMCONTROLLER_ICONS / "icon_equals.png"
        icon_settings = generate_settings_icon()
        icon_num = generate_task_number_icon(current_idx, max(1, total_projects))

        # Komendy
        new_task_cmd = '$HOME/.local/bin/sd_algo_new_task.sh'
        settings_cmd = '$HOME/.local/bin/sd_algo_panel.sh'
        test_cmd = f'$HOME/.local/bin/sd_algo_test_vscode.sh "{pdir}" "{problem_id}"'
        run_cmd = f'$HOME/.local/bin/sd_algo_run_vscode.sh "{pdir}" "{problem_id}"'
        kill_cmd = f'bash "{pdir}/.algo/kill.sh"'
        menu_cmd = f'$HOME/.local/bin/sd_algo_switch.sh menu "{problem_id}"'
        prev_cmd = f'$HOME/.local/bin/sd_algo_switch.sh prev "{problem_id}"'
        next_cmd = f'$HOME/.local/bin/sd_algo_switch.sh next "{problem_id}"'

        if total_projects <= 1:
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
                "labels": {},
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

        info_cmd = f'notify-send "AlgoDeck" "Zadanie {current_idx}/{total_projects}: {analysis.get("title", prob_id)} [{prob_id}]" 2>/dev/null || true'

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

                # RZĄD 2 (DÓŁ): [ ⚙ ] [   ] [ ‹ ] [Nr ] [ › ]
                "0x2": make_key(settings_cmd, icon_settings),
                "1x2": make_empty_key(),
                "2x2": make_key(left_cmd, icon_left),
                "3x2": make_key(info_cmd, icon_num),
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

    def get_active_page_name(self) -> str:
        """Pobiera aktualnie wyświetlaną stronę ze StreamControllera."""
        try:
            import re
            res = subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", f"/com/core447/StreamController/controllers/{SERIAL}",
                "--method", "org.freedesktop.DBus.Properties.Get",
                "com.core447.StreamController.Controller", "ActivePageName"
            ], timeout=2, capture_output=True, text=True)
            if res.returncode == 0:
                m = re.search(r"<'([^']+)'", res.stdout)
                if m:
                    return m.group(1)
        except Exception:
            pass
        return ""

    def sync_all_problems(self):
        """Automatycznie generuje i synchronizuje czyste profile dla wszystkich zadań w workspace."""
        self.ensure_vector_icons()
        if not self.workspace_dir.exists():
            self.generate_idle_page(switch_now=True)
            return
        
        # Zapamiętaj aktualną stronę lub ostatnio aktywne zadanie
        saved_page = self.get_active_page_name()
        if not saved_page or saved_page in ("Main", "ALGO_IDLE"):
            if Path("/tmp/algodeck_active_task.txt").exists():
                try:
                    saved_page = Path("/tmp/algodeck_active_task.txt").read_text(encoding="utf-8").strip()
                except Exception:
                    pass

        ordered_pids = self.get_ordered_problem_ids()
        count = len(ordered_pids)
        first_prob_id = ordered_pids[0].upper() if ordered_pids else ""

        for pid in ordered_pids:
            p = self.workspace_dir / pid
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
        self.generate_idle_page(switch_now=(count == 0))

        if count > 0:
            target = saved_page if (saved_page and saved_page not in ("Main", "ALGO_IDLE")) else first_prob_id
            if target:
                self.switch_to_page(target)
        else:
            self.switch_to_page("ALGO_IDLE")

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

        # Zbierz wszystkie zadania w ścisłej kolejności chronologicznej utworzenia
        ordered_pids = self.get_ordered_problem_ids()
        projects: List[Dict[str, str]] = []
        for pid in ordered_pids:
            p = self.workspace_dir / pid
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
        if prob_id != "ALGO_IDLE" and not prob_id.startswith("ALGO_MENU"):
            try:
                Path("/tmp/algodeck_active_task.txt").write_text(prob_id, encoding="utf-8")
            except Exception:
                pass
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
