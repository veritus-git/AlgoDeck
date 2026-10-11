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


def make_empty_key() -> Dict[str, Any]:
    return {
        "states": {
            "0": {
                "actions": [],
                "labels": {},
                "background": {"color": [0, 0, 0, 255]},
                "image-control-action": 0,
                "label-control-actions": [0, 0, 0],
                "background-control-action": 0,
            }
        }
    }


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
        Upewnia się, że oficjalny plugin com_core447_OSPlugin jest zainstalowany w StreamControllerze,
        ma nadane uprawnienia host spawn w Flatpaku oraz jest aktywowany bez konieczności klikania w GUI.
        """
        # 1. Nadaj uprawnienia host spawn dla Flatpaka StreamController
        try:
            subprocess.run(["flatpak", "override", "--user", "--talk-name=org.freedesktop.Flatpak", "com.core447.StreamController"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            subprocess.run(["flatpak", "override", "--user", "--filesystem=host", "com.core447.StreamController"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except Exception:
            pass

        plugin_dir = Path.home() / ".var/app/com.core447.StreamController/data/plugins/com_core447_OSPlugin"
        manifest_file = plugin_dir / "manifest.json"

        if not manifest_file.exists():
            logger.info("Instalacja brakującego pluginu com_core447_OSPlugin w StreamControllerze...")
            plugin_dir.mkdir(parents=True, exist_ok=True)

            # Kopiuj z lokalnych zasobów offline repozytorium
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

            # Pobierz z sieci jeśli lokalna kopia nie była dostępna
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

        # 2. Aktywuj plugin live przez D-Bus jeśli StreamController działa (usuwa żółte kropki natychmiast)
        try:
            subprocess.run([
                "gdbus", "call", "--session", "--dest", "com.core447.StreamController",
                "--object-path", "/com/core447/StreamController",
                "--method", "org.gtk.Actions.Activate", "install-plugin", "[<'com_core447_OSPlugin'>]", "{}"
            ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except Exception:
            pass

        return manifest_file.exists()

    def ensure_vector_icons(self):
        """Upewnia się, że geometryczne ikony na czarnym tle, plugin OS oraz integracja VS Code istnieją."""
        self.ensure_os_plugin()
        try:
            from backend.workspace.builder import WorkspaceBuilder
            WorkspaceBuilder.ensure_vscode_environment()
        except Exception:
            pass
        STREAMCONTROLLER_ICONS.mkdir(parents=True, exist_ok=True)
        try:
            from backend.streamdeck.icons_generator import generate_all_base_icons
            generate_all_base_icons()
        except Exception as e:
            logger.warning(f"Błąd podczas sprawdzania ikon: {e}")

    def sync_page_to_streamcontroller(self, page_name: str, page_data: Dict[str, Any]):
        """
        Zapisuje stronę na dysk w katalogu stron StreamControllera.
        Wymusza natychmiastowe przeładowanie w pamięci działającego StreamControllera przez DBus (RemovePage + AddPage).
        Nigdy nie pomija DBus, aby zapobiec rozsynchronizowaniu pamięci procesu z plikami na dysku.
        """
        prob_id = page_name.upper()
        json_str = json.dumps(page_data, indent=4)
        target_page_path = STREAMCONTROLLER_PAGES / f"{prob_id}.json"

        try:
            # 1. Zawsze usuń starą stronę z pamięci StreamControllera
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

            # 2. Zapisz zaktualizowany plik na dysku
            target_page_path.write_text(json_str, encoding="utf-8")
            logger.info(f"Pomyślnie zapisano stronę {prob_id} dla StreamControllera.")

            # 3. Załaduj definicję do pamięci StreamControllera przez AddPage
            try:
                gvariant_str = json.dumps(json_str)
                subprocess.run([
                    "gdbus", "call", "--session",
                    "--dest", "com.core447.StreamController",
                    "--object-path", "/com/core447/StreamController",
                    "--method", "com.core447.StreamController.AddPage",
                    prob_id, gvariant_str
                ], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            except Exception:
                pass
        except Exception as e:
            logger.error(f"Nie udało się zapisać pliku {target_page_path}: {e}")

    def generate_idle_page(self, switch_now: bool = False):
        """
        Generuje 'Ekran zachęty' (ALGO_IDLE) zawierający JEDYNIE logo na środku:
        - Rząd 0: Wszystkie 5 przycisków całkowicie puste
        - Rząd 1: [   ] [ ALGO ] [ ⚡ ] [ DECK ] [   ] (Środkowe 3 przyciski to logo)
        - Rząd 2: Wszystkie 5 przycisków całkowicie puste
        Zero strzałek, zero trybików, zero zbędnych ikon.
        """
        self.ensure_vector_icons()

        icon_algo = STREAMCONTROLLER_ICONS / "idle_logo_algo.png"
        icon_bolt = STREAMCONTROLLER_ICONS / "idle_logo_bolt.png"
        icon_deck = STREAMCONTROLLER_ICONS / "idle_logo_deck.png"

        start_cmd = '$HOME/.local/bin/sd_algo_panel.sh --tab=new'

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

        page_data = {
            "screensaver": {},
            "keys": {
                # RZĄD 0 (GÓRA): Całkowicie pusty
                "0x0": make_empty_key(),
                "1x0": make_empty_key(),
                "2x0": make_empty_key(),
                "3x0": make_empty_key(),
                "4x0": make_empty_key(),

                # RZĄD 1 (ŚRODEK): JEDYNIE LOGO (ALGO, ⚡, DECK) na środkowych 3 przyciskach
                "0x1": make_empty_key(),
                "1x1": make_key(start_cmd, icon_algo),
                "2x1": make_key(start_cmd, icon_bolt),
                "3x1": make_key(start_cmd, icon_deck),
                "4x1": make_empty_key(),

                # RZĄD 2 (DÓŁ): Całkowicie pusty
                "0x2": make_empty_key(),
                "1x2": make_empty_key(),
                "2x2": make_empty_key(),
                "3x2": make_empty_key(),
                "4x2": make_empty_key(),
            }
        }
        self.sync_page_to_streamcontroller("ALGO_IDLE", page_data)
        if switch_now:
            try:
                prev_file = Path(os.path.expanduser("~/.config/algodeck/prev_page.txt"))
                prev_file.parent.mkdir(parents=True, exist_ok=True)
                if not prev_file.exists():
                    res = subprocess.run([
                        "gdbus", "call", "--session",
                        "--dest", "com.core447.StreamController",
                        "--object-path", "/com/core447/StreamController",
                        "--method", "org.freedesktop.DBus.Properties.Get",
                        "com.core447.StreamController", "CurrentPage"
                    ], timeout=1, capture_output=True, text=True)
                    if res.returncode == 0 and res.stdout:
                        import re
                        m = re.search(r"'([^']+)'", res.stdout)
                        if m and not m.group(1).startswith("ALGO_"):
                            prev_file.write_text(m.group(1))
            except Exception:
                pass
            self.switch_to_page("ALGO_IDLE")

    def generate_page_for_problem(self, problem_id: str, analysis: Optional[Dict[str, Any]] = None, switch_now: bool = True) -> bool:
        """
        Zapisuje stronę zadania w StreamControllerze:
        - Rząd 0: [ = ] [ L1 ] [ L2 ] [ L3 ] [ = ]
        - Rząd 1: [ + ] [ ✓ ] [ ▶ ] [ ⏹ ] [ ⊞ ]
        - Rząd 2: [ ⚙ ] [ ↩ ] [ ‹ ] [Nr ] [ › ] (Ustawienia, Wyjście, Lewo, Nr zadania, Prawo)
        """
        prob_id = problem_id.upper()
        pdir = (self.workspace_dir / problem_id.lower()).resolve()
        analysis = analysis or {}

        STREAMCONTROLLER_PAGES.mkdir(parents=True, exist_ok=True)
        self.ensure_vector_icons()

        from backend.streamdeck.icons_generator import (
            generate_letter_icon, generate_settings_icon, generate_task_number_icon,
            generate_exit_curved_arrow_icon
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

    def sync_all_problems(self, default_to_idle: bool = True):
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
        self.generate_idle_page(switch_now=False)

        valid_pids_upper = [p.upper() for p in ordered_pids]
        if saved_page and saved_page.upper() in valid_pids_upper:
            self.switch_to_page(saved_page)
        elif not default_to_idle and first_prob_id:
            self.switch_to_page(first_prob_id)
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

            # Pre-inicjalizacja wszystkich 15 slotów (5x3) jako puste, aby nie zostawały stare/widmowe przyciski
            keys: Dict[str, Any] = {
                f"{x}x{y}": make_empty_key() for x in range(5) for y in range(3)
            }

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

            # 2b. Slot (1x2) pozostaje pusty (wyjście przeniesione do zasobnika systemowego)
            keys["1x2"] = make_empty_key()

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

    def ensure_streamcontroller_running(self):
        """Upewnia się, że StreamController (lub oficjalny Stream Deck na macOS/Win) jest uruchomiony w tle i gotowy do przyjmowania poleceń."""
        import sys
        import time
        if sys.platform.startswith("linux"):
            try:
                res = subprocess.run(["pgrep", "-f", "StreamController"], capture_output=True)
                if res.returncode != 0:
                    logger.info("StreamController nie jest uruchomiony. Uruchamiam w tle...")
                    launched = False
                    if shutil.which("flatpak"):
                        try:
                            chk = subprocess.run(["flatpak", "info", "com.core447.StreamController"], capture_output=True)
                            if chk.returncode == 0:
                                env = dict(os.environ)
                                if "DISPLAY" not in env or not env["DISPLAY"]:
                                    env["DISPLAY"] = ":0"
                                subprocess.Popen(["flatpak", "run", "com.core447.StreamController"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                                launched = True
                        except Exception:
                            pass
                    if not launched and shutil.which("streamcontroller"):
                        env = dict(os.environ)
                        if "DISPLAY" not in env or not env["DISPLAY"]:
                            env["DISPLAY"] = ":0"
                        subprocess.Popen(["streamcontroller"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                        launched = True

                    # Czekaj na rejestrację na DBus (max 6 sekund)
                    for _ in range(30):
                        time.sleep(0.2)
                        chk_dbus = subprocess.run([
                            "gdbus", "call", "--session",
                            "--dest", "com.core447.StreamController",
                            "--object-path", "/com/core447/StreamController",
                            "--method", "org.freedesktop.DBus.Peer.Ping"
                        ], capture_output=True)
                        if chk_dbus.returncode == 0:
                            logger.info("StreamController jest gotowy i odpowiada na DBus.")
                            break
            except Exception as e:
                logger.warning(f"Błąd uruchamiania StreamControllera: {e}")
        elif sys.platform == "darwin":
            try:
                res = subprocess.run(["pgrep", "-f", "Stream Deck"], capture_output=True)
                if res.returncode != 0:
                    subprocess.Popen(["open", "-a", "Elgato Stream Deck"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    time.sleep(1.5)
            except Exception:
                pass
        elif sys.platform == "win32":
            try:
                res = subprocess.run(["tasklist", "/FI", "IMAGENAME eq StreamDeck.exe"], capture_output=True, text=True)
                if "StreamDeck.exe" not in res.stdout:
                    sd_exe = Path(os.environ.get("ProgramFiles", "C:\\Program Files")) / "Elgato" / "StreamDeck" / "StreamDeck.exe"
                    if sd_exe.exists():
                        subprocess.Popen([str(sd_exe)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                        time.sleep(1.5)
            except Exception:
                pass

    def cleanup_and_exit(self):
        """
        Całkowite wyłączenie AlgoDeck:
        1. Odczytuje domyślną stronę użytkownika (np. 'Main') i natychmiast przełącza StreamController z powrotem na nią.
        2. Pobiera listę stron ze StreamControllera przez DBus oraz katalogu workspace.
        3. Usuwa wszystkie profile zadań i menu ze StreamControllera przez DBus RemovePage oraz z dysku.
        4. Czyści plik /tmp/algodeck_active_task.txt.
        """
        default_page = "Main"
        pages_settings = STREAMCONTROLLER_DATA / "settings/pages.json"
        if pages_settings.exists():
            try:
                pdata = json.loads(pages_settings.read_text(encoding="utf-8"))
                defs = pdata.get("default-pages", {})
                if SERIAL in defs:
                    default_page = Path(defs[SERIAL]).stem
                elif defs:
                    default_page = Path(next(iter(defs.values()))).stem
            except Exception:
                pass

        # 1. Przełącz na poprzednią stronę użytkownika (lub domyślną)
        prev_file = Path(os.path.expanduser("~/.config/algodeck/prev_page.txt"))
        target_restore = default_page
        if prev_file.exists():
            try:
                saved = prev_file.read_text().strip()
                if saved and not saved.startswith("ALGO_"):
                    target_restore = saved
            except Exception:
                pass

        try:
            subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", "/com/core447/StreamController",
                "--method", "com.core447.StreamController.ChangePage",
                SERIAL, target_restore
            ], timeout=2, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except Exception:
            pass

        # 2. Zbierz WYŁĄCZNIE strony AlgoDeck do usunięcia (nigdy stron użytkownika!)
        ws_problems = set()
        ws_dir = settings.workspace_dir
        if ws_dir.exists():
            for p in ws_dir.iterdir():
                if p.is_dir() and not p.name.startswith("."):
                    ws_problems.add(p.name.upper())

        pages_to_remove = ["ALGO_IDLE", "ALGO_MENU"]
        # Dodaj problem pages
        pages_to_remove.extend(list(ws_problems))

        # Sprawdź strony ze StreamControllera przez właściwość DBus Pages
        try:
            import re
            res = subprocess.run([
                "gdbus", "call", "--session",
                "--dest", "com.core447.StreamController",
                "--object-path", "/com/core447/StreamController",
                "--method", "org.freedesktop.DBus.Properties.Get",
                "com.core447.StreamController", "Pages"
            ], timeout=2, capture_output=True, text=True)
            if res.returncode == 0:
                found_pages = re.findall(r"'([^']+)'", res.stdout)
                for p in found_pages:
                    # Tylko i wyłącznie strony AlgoDeck!
                    if p.startswith("ALGO_") or p in ws_problems:
                        pages_to_remove.append(p)
        except Exception:
            pass

        if STREAMCONTROLLER_PAGES.exists():
            for pf in STREAMCONTROLLER_PAGES.glob("*.json"):
                stem = pf.stem
                if stem.startswith("ALGO_") or stem in ws_problems:
                    pages_to_remove.append(stem)

        # Wyklucz prywatne profile użytkownika
        protected_pages = {"MAIN", "DEFAULT", "DEV & SECRETS", "PULPIT 1 - DEV", "PULPIT 2 - PERSONAL"}
        pages_to_remove = [p for p in set(pages_to_remove) if p.upper() not in protected_pages]

        # 3. Usuń każdą stronę AlgoDeck przez DBus i usuń plik JSON
        for p in pages_to_remove:
            try:
                subprocess.run([
                    "gdbus", "call", "--session",
                    "--dest", "com.core447.StreamController",
                    "--object-path", "/com/core447/StreamController",
                    "--method", "com.core447.StreamController.RemovePage",
                    p
                ], timeout=1, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            except Exception:
                pass

            json_file = STREAMCONTROLLER_PAGES / f"{p}.json"
            if json_file.exists():
                try:
                    json_file.unlink()
                except Exception:
                    pass

        try:
            if Path("/tmp/algodeck_active_task.txt").exists():
                Path("/tmp/algodeck_active_task.txt").unlink()
        except Exception:
            pass

        logger.info(f"Usunięto profile AlgoDeck ({pages_to_remove}) ze StreamControllera i przywrócono stronę {default_page}.")

    def switch_to_page(self, page_name: str) -> bool:
        """Wysyła sygnał DBus do StreamControllera, by natychmiast wyświetlić stronę na fizycznym Stream Decku."""
        self.ensure_streamcontroller_running()
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
