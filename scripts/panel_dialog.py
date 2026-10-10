#!/usr/bin/env python3
"""
AlgoDeck ⚡ Koło MAP - Główny Panel Kontrolny i Ustawienia
Natywna aplikacja desktopowa w czystym, głębokim motywie Cyberpunk/Dark:
- ZERO białych ramek, ZERO brzydkich systemowych kontrolek X11
- Własne segmentowane przyciski (Radio/Toggle buttons bez kółek i ramek)
- 3 Zakładki: '➕ Nowe Zadanie', '📋 Zadania', '⚙️ Ustawienia'
- Automatyczne wczytywanie pliku PDF podanego jako argument --pdf
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import urllib.request
import urllib.error
from pathlib import Path
from typing import Dict, Any, List, Optional

import tkinter as tk
from tkinter import messagebox, filedialog

API_BASE = "http://127.0.0.1:8080"
CONFIG_FILE = Path.home() / ".config/algodeck/settings.json"

def get_primary_monitor_geometry():
    """Wykrywa geometrię głównego monitora z xrandr."""
    try:
        res = subprocess.run(["xrandr", "--current"], capture_output=True, text=True, timeout=2)
        if res.returncode == 0:
            match = re.search(r'connected\s+primary\s+(\d+)x(\d+)\+(\d+)\+(\d+)', res.stdout)
            if match:
                return tuple(map(int, match.groups()))
            match = re.search(r'connected\s+(?:primary\s+)?(\d+)x(\d+)\+(\d+)\+(\d+)', res.stdout)
            if match:
                return tuple(map(int, match.groups()))
    except Exception:
        pass
    return None

def load_user_settings() -> Dict[str, Any]:
    defaults = {
        "workspace_dir": str(Path.home() / "algodeck-workspace"),
        "vscode_mode": "single_window",
        "notifications": True
    }
    if CONFIG_FILE.exists():
        try:
            saved = json.loads(CONFIG_FILE.read_text(encoding="utf-8"))
            defaults.update(saved)
        except Exception:
            pass
    return defaults

def save_user_settings(settings: Dict[str, Any]):
    CONFIG_FILE.parent.mkdir(parents=True, exist_ok=True)
    CONFIG_FILE.write_text(json.dumps(settings, indent=2, ensure_ascii=False), encoding="utf-8")

class AlgoDeckPanel:
    def __init__(self, root: tk.Tk, initial_tab: str = "new", initial_pdf: str = ""):
        self.root = root
        self.root.title("AlgoDeck ⚡ Centrum Kontroli")
        self.root.configure(bg="#080b11")
        self.root.resizable(False, False)

        self.settings = load_user_settings()
        self.problems_cache: List[Dict[str, Any]] = []
        self.active_problem_id = ""

        # Wymiary okna
        w = 600
        h = 660

        geom = get_primary_monitor_geometry()
        if geom:
            mw, mh, mx, my = geom
            x = mx + (mw - w) // 2
            y = my + (mh - h) // 2
        else:
            sw = self.root.winfo_screenwidth()
            sh = self.root.winfo_screenheight()
            x = (sw - w) // 2
            y = (sh - h) // 2

        self.root.geometry(f"{w}x{h}+{x}+{y}")
        self.root.attributes("-topmost", True)
        self.root.bind("<Escape>", lambda e: self.root.destroy())

        self._build_header()
        self._build_tabs()
        self._build_panes()

        self.switch_tab(initial_tab)

        if initial_pdf and os.path.exists(initial_pdf):
            self.set_pdf_file(initial_pdf)

    # ----------------- Nagłówek -----------------
    def _build_header(self):
        header = tk.Frame(self.root, bg="#080b11", padx=22, pady=12, highlightthickness=0)
        header.pack(fill="x")

        lbl_logo = tk.Label(header, text="⚡", font=("Segoe UI", 18), bg="#080b11", fg="#38bdf8", highlightthickness=0)
        lbl_logo.pack(side="left", padx=(0, 8))

        lbl_title = tk.Label(header, text="ALGODECK", font=("Segoe UI", 16, "bold"), bg="#080b11", fg="#f8fafc", highlightthickness=0)
        lbl_title.pack(side="left")

        lbl_sub = tk.Label(header, text="KOŁO MAP • OIJ • OI", font=("Segoe UI", 9, "bold"), bg="#080b11", fg="#38bdf8", highlightthickness=0)
        lbl_sub.pack(side="right")

    # ----------------- Nawigacja Zakładek -----------------
    def _build_tabs(self):
        tab_bar = tk.Frame(self.root, bg="#0f1420", padx=16, pady=4, highlightthickness=0)
        tab_bar.pack(fill="x")

        self.tab_buttons = {}
        tabs = [
            ("new", "➕ Nowe Zadanie"),
            ("tasks", "📋 Zadania"),
            ("settings", "⚙️ Ustawienia")
        ]

        for tab_id, text in tabs:
            btn = tk.Button(
                tab_bar, text=text, font=("Segoe UI", 10, "bold"),
                bg="#0f1420", fg="#94a3b8", activebackground="#1e293b", activeforeground="#f8fafc",
                relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=16, pady=8,
                command=lambda t=tab_id: self.switch_tab(t)
            )
            btn.pack(side="left", padx=(0, 6))
            self.tab_buttons[tab_id] = btn

    # ----------------- Kontenery Treści Zakładek -----------------
    def _build_panes(self):
        self.pane_container = tk.Frame(self.root, bg="#080b11", padx=20, pady=12, highlightthickness=0)
        self.pane_container.pack(fill="both", expand=True)

        self.panes = {
            "new": tk.Frame(self.pane_container, bg="#080b11", highlightthickness=0),
            "tasks": tk.Frame(self.pane_container, bg="#080b11", highlightthickness=0),
            "settings": tk.Frame(self.pane_container, bg="#080b11", highlightthickness=0)
        }

        self._build_tab_new(self.panes["new"])
        self._build_tab_tasks(self.panes["tasks"])
        self._build_tab_settings(self.panes["settings"])

    def switch_tab(self, tab_id: str):
        for tid, btn in self.tab_buttons.items():
            if tid == tab_id:
                btn.config(bg="#1e293b", fg="#38bdf8")
            else:
                btn.config(bg="#0f1420", fg="#94a3b8")

        for tid, pane in self.panes.items():
            if tid == tab_id:
                pane.pack(fill="both", expand=True)
            else:
                pane.pack_forget()

        if tab_id == "tasks":
            self.refresh_tasks_list()

    # ----------------- Zakładka 1: Nowe Zadanie -----------------
    def _build_tab_new(self, parent):
        # Segmentowy przełącznik trybu (CAŁKOWICIE BEZ BIAŁYCH RAMEK I KÓŁEK)
        seg_frame = tk.Frame(parent, bg="#080b11", highlightthickness=0)
        seg_frame.pack(fill="x", pady=(0, 10))

        self.current_new_mode = "pdf"

        self.btn_mode_pdf = tk.Button(
            seg_frame, text="📄 Z pliku PDF (Olimpijskie)", font=("Segoe UI", 9, "bold"),
            bg="#0284c7", fg="#ffffff", activebackground="#0369a1", activeforeground="#ffffff",
            relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=14, pady=6,
            command=lambda: self._set_new_mode("pdf")
        )
        self.btn_mode_pdf.pack(side="left", padx=(0, 8))

        self.btn_mode_manual = tk.Button(
            seg_frame, text="✏️ Ręczne wpisanie", font=("Segoe UI", 9, "bold"),
            bg="#131b2c", fg="#94a3b8", activebackground="#1e293b", activeforeground="#f8fafc",
            relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=14, pady=6,
            command=lambda: self._set_new_mode("manual")
        )
        self.btn_mode_manual.pack(side="left")

        # Karta formularza
        self.card_new = tk.Frame(parent, bg="#111726", padx=14, pady=12, highlightthickness=0)
        self.card_new.pack(fill="both", expand=True, pady=(0, 12))

        # Sekcja PDF
        self.frame_pdf_inputs = tk.Frame(self.card_new, bg="#111726", highlightthickness=0)
        self.frame_pdf_inputs.pack(fill="x", pady=(0, 6))

        tk.Label(self.frame_pdf_inputs, text="Plik PDF z treścią zadania:", font=("Segoe UI", 9, "bold"), bg="#111726", fg="#94a3b8", highlightthickness=0).pack(anchor="w")

        row_pdf = tk.Frame(self.frame_pdf_inputs, bg="#111726", highlightthickness=0)
        row_pdf.pack(fill="x", pady=(2, 6))

        self.entry_pdf_path = tk.Entry(row_pdf, bg="#182238", fg="#f8fafc", insertbackground="#38bdf8", relief="flat", bd=0, highlightthickness=0, font=("Consolas", 10))
        self.entry_pdf_path.pack(side="left", fill="x", expand=True, padx=(0, 6), ipady=5)

        btn_browse_pdf = tk.Button(
            row_pdf, text="Przeglądaj...", font=("Segoe UI", 9), bg="#1e2d4a", fg="#e2e8f0",
            activebackground="#2b3e66", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=12, pady=4, command=self._browse_pdf
        )
        btn_browse_pdf.pack(side="left")

        # Opcjonalny plik ZIP
        tk.Label(self.frame_pdf_inputs, text="Paczka testów ZIP (opcjonalna):", font=("Segoe UI", 9, "bold"), bg="#111726", fg="#94a3b8", highlightthickness=0).pack(anchor="w")

        row_zip = tk.Frame(self.frame_pdf_inputs, bg="#111726", highlightthickness=0)
        row_zip.pack(fill="x", pady=(2, 6))

        self.entry_zip_path = tk.Entry(row_zip, bg="#182238", fg="#f8fafc", insertbackground="#38bdf8", relief="flat", bd=0, highlightthickness=0, font=("Consolas", 10))
        self.entry_zip_path.pack(side="left", fill="x", expand=True, padx=(0, 6), ipady=5)

        btn_browse_zip = tk.Button(
            row_zip, text="Przeglądaj...", font=("Segoe UI", 9), bg="#1e2d4a", fg="#e2e8f0",
            activebackground="#2b3e66", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=12, pady=4, command=self._browse_zip
        )
        btn_browse_zip.pack(side="left")

        # Wspólne pola zadania
        tk.Label(self.card_new, text="Nazwa / Kod zadania * (np. chw, kol, świ):", font=("Segoe UI", 9, "bold"), bg="#111726", fg="#94a3b8", highlightthickness=0).pack(anchor="w")
        self.entry_new_code = tk.Entry(self.card_new, bg="#182238", fg="#f8fafc", insertbackground="#38bdf8", relief="flat", bd=0, highlightthickness=0, font=("Consolas", 11))
        self.entry_new_code.pack(fill="x", pady=(2, 6), ipady=4)

        tk.Label(self.card_new, text="Pełny tytuł (opcjonalny):", font=("Segoe UI", 9, "bold"), bg="#111726", fg="#94a3b8", highlightthickness=0).pack(anchor="w")
        self.entry_new_title = tk.Entry(self.card_new, bg="#182238", fg="#f8fafc", insertbackground="#38bdf8", relief="flat", bd=0, highlightthickness=0, font=("Consolas", 11))
        self.entry_new_title.pack(fill="x", pady=(2, 6), ipady=4)

        # Wiersz z limitami
        row_lim = tk.Frame(self.card_new, bg="#111726", highlightthickness=0)
        row_lim.pack(fill="x", pady=(0, 6))
        row_lim.columnconfigure(0, weight=1, uniform="lim")
        row_lim.columnconfigure(1, weight=1, uniform="lim")

        col_t = tk.Frame(row_lim, bg="#111726", highlightthickness=0)
        col_t.grid(row=0, column=0, sticky="ew", padx=(0, 6))
        tk.Label(col_t, text="Czas (s):", font=("Segoe UI", 9, "bold"), bg="#111726", fg="#94a3b8", highlightthickness=0).pack(anchor="w")
        self.entry_new_time = tk.Entry(col_t, bg="#182238", fg="#f8fafc", insertbackground="#38bdf8", relief="flat", bd=0, highlightthickness=0, font=("Consolas", 10))
        self.entry_new_time.insert(0, "1.0")
        self.entry_new_time.pack(fill="x", pady=(2, 0), ipady=4)

        col_m = tk.Frame(row_lim, bg="#111726", highlightthickness=0)
        col_m.grid(row=0, column=1, sticky="ew", padx=(6, 0))
        tk.Label(col_m, text="RAM (MB):", font=("Segoe UI", 9, "bold"), bg="#111726", fg="#94a3b8", highlightthickness=0).pack(anchor="w")
        self.entry_new_mem = tk.Entry(col_m, bg="#182238", fg="#f8fafc", insertbackground="#38bdf8", relief="flat", bd=0, highlightthickness=0, font=("Consolas", 10))
        self.entry_new_mem.insert(0, "128")
        self.entry_new_mem.pack(fill="x", pady=(2, 0), ipady=4)

        # Sekcja testów ręcznych
        self.frame_manual_tests = tk.Frame(self.card_new, bg="#111726", highlightthickness=0)
        tk.Label(self.frame_manual_tests, text="Przykładowe wejście cin (opcjonalne):", font=("Segoe UI", 9, "bold"), bg="#111726", fg="#94a3b8", highlightthickness=0).pack(anchor="w")
        self.text_manual_in = tk.Text(self.frame_manual_tests, height=2, bg="#182238", fg="#f8fafc", insertbackground="#38bdf8", relief="flat", bd=0, highlightthickness=0, font=("Consolas", 9))
        self.text_manual_in.pack(fill="x", pady=(2, 4))

        tk.Label(self.frame_manual_tests, text="Oczekiwane wyjście cout (opcjonalne):", font=("Segoe UI", 9, "bold"), bg="#111726", fg="#94a3b8", highlightthickness=0).pack(anchor="w")
        self.text_manual_out = tk.Text(self.frame_manual_tests, height=2, bg="#182238", fg="#f8fafc", insertbackground="#38bdf8", relief="flat", bd=0, highlightthickness=0, font=("Consolas", 9))
        self.text_manual_out.pack(fill="x", pady=(2, 4))

        # Przycisk główny (Zero białych ramek)
        self.btn_submit_create = tk.Button(
            parent, text="⚡ Utwórz Workspace i Otwórz VS Code", font=("Segoe UI", 11, "bold"),
            bg="#0284c7", fg="#ffffff", activebackground="#0369a1", activeforeground="#ffffff",
            relief="flat", bd=0, highlightthickness=0, cursor="hand2", pady=10, command=self._submit_create_task
        )
        self.btn_submit_create.pack(fill="x")

    def _set_new_mode(self, mode: str):
        self.current_new_mode = mode
        if mode == "pdf":
            self.btn_mode_pdf.config(bg="#0284c7", fg="#ffffff")
            self.btn_mode_manual.config(bg="#131b2c", fg="#94a3b8")
            self.frame_manual_tests.pack_forget()
            self.frame_pdf_inputs.pack(fill="x", pady=(0, 6))
        else:
            self.btn_mode_manual.config(bg="#0284c7", fg="#ffffff")
            self.btn_mode_pdf.config(bg="#131b2c", fg="#94a3b8")
            self.frame_pdf_inputs.pack_forget()
            self.frame_manual_tests.pack(fill="x", pady=(4, 0))

    def set_pdf_file(self, file_path: str):
        self._set_new_mode("pdf")
        self.entry_pdf_path.delete(0, "end")
        self.entry_pdf_path.insert(0, file_path)
        self._auto_parse_pdf(file_path)

    def _browse_pdf(self):
        f = filedialog.askopenfilename(
            title="Wybierz plik zadania PDF",
            filetypes=[("Dokumenty PDF", "*.pdf"), ("Wszystkie pliki", "*.*")]
        )
        if f:
            self.set_pdf_file(f)

    def _browse_zip(self):
        f = filedialog.askopenfilename(
            title="Wybierz archiwum ZIP z testami",
            filetypes=[("Archiwa ZIP", "*.zip"), ("Wszystkie pliki", "*.*")]
        )
        if f:
            self.entry_zip_path.delete(0, "end")
            self.entry_zip_path.insert(0, f)

    def _auto_parse_pdf(self, pdf_path: str):
        try:
            payload = json.dumps({"file_path": pdf_path, "auto_create": False}).encode("utf-8")
            req = urllib.request.Request(f"{API_BASE}/api/auto-import-pdf", data=payload, headers={"Content-Type": "application/json"})
            with urllib.request.urlopen(req, timeout=3) as res:
                if res.status == 200:
                    data = json.loads(res.read().decode("utf-8"))
                    if data.get("problem_id") and not self.entry_new_code.get().strip():
                        self.entry_new_code.delete(0, "end")
                        self.entry_new_code.insert(0, data["problem_id"])
                    if data.get("title") and not self.entry_new_title.get().strip():
                        self.entry_new_title.delete(0, "end")
                        self.entry_new_title.insert(0, data["title"])
                    if data.get("time_limit_sec"):
                        self.entry_new_time.delete(0, "end")
                        self.entry_new_time.insert(0, str(data["time_limit_sec"]))
                    if data.get("memory_limit_mb"):
                        self.entry_new_mem.delete(0, "end")
                        self.entry_new_mem.insert(0, str(data["memory_limit_mb"]))
        except Exception:
            stem = Path(pdf_path).stem.lower()
            if not self.entry_new_code.get().strip():
                self.entry_new_code.delete(0, "end")
                self.entry_new_code.insert(0, stem[:5])

    def _submit_create_task(self):
        code = self.entry_new_code.get().strip().lower()
        if not code:
            messagebox.showwarning("AlgoDeck", "Podaj kod/nazwę zadania!")
            self.entry_new_code.focus_set()
            return

        title = self.entry_new_title.get().strip() or code.capitalize()
        try:
            t_lim = float(self.entry_new_time.get().strip())
        except ValueError:
            t_lim = 1.0

        try:
            m_lim = int(self.entry_new_mem.get().strip())
        except ValueError:
            m_lim = 128

        self.btn_submit_create.config(text="Tworzenie workspace...", state="disabled")
        self.root.update()

        try:
            if self.current_new_mode == "pdf":
                pdf_p = self.entry_pdf_path.get().strip()
                zip_p = self.entry_zip_path.get().strip()
                if pdf_p and os.path.exists(pdf_p):
                    payload = {
                        "file_path": pdf_p,
                        "problem_id": code,
                        "title": title,
                        "time_limit": t_lim,
                        "memory_limit": m_lim,
                        "auto_create": True
                    }
                    data = json.dumps(payload).encode("utf-8")
                    req = urllib.request.Request(f"{API_BASE}/api/auto-import-pdf", data=data, headers={"Content-Type": "application/json"})
                    urllib.request.urlopen(req, timeout=8)

                    if zip_p and os.path.exists(zip_p):
                        self._upload_zip_tests(code, zip_p)
                else:
                    self._send_create_manual(code, title, t_lim, m_lim, [])
            else:
                tin = self.text_manual_in.get("1.0", "end-1c").strip()
                tout = self.text_manual_out.get("1.0", "end-1c").strip()
                tests = []
                if tin or tout:
                    tests.append({"id": "test_1", "name": "Przykład 1", "input": tin, "expected_output": tout})
                self._send_create_manual(code, title, t_lim, m_lim, tests)

            os.system(f'notify-send "AlgoDeck" "Utworzono zadanie {code.upper()}! Otwarto VS Code." 2>/dev/null || true')
            self.root.destroy()
        except Exception as e:
            messagebox.showerror("Błąd", f"Nie udało się utworzyć zadania:\n{e}")
            self.btn_submit_create.config(text="⚡ Utwórz Workspace i Otwórz VS Code", state="normal")

    def _send_create_manual(self, code: str, title: str, t_lim: float, m_lim: int, tests: List[Dict]):
        payload = {
            "problem_id": code,
            "title": title,
            "time_limit": t_lim,
            "memory_limit": m_lim,
            "tests": tests
        }
        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(f"{API_BASE}/api/create-manual", data=data, headers={"Content-Type": "application/json"})
        urllib.request.urlopen(req, timeout=5)

    def _upload_zip_tests(self, problem_id: str, zip_path: str):
        boundary = "----AlgoDeckBoundaryZip998877"
        with open(zip_path, "rb") as zf:
            zip_bytes = zf.read()

        body = (
            f"--{boundary}\r\n"
            f'Content-Disposition: form-data; name="file"; filename="{Path(zip_path).name}"\r\n'
            f"Content-Type: application/zip\r\n\r\n"
        ).encode("utf-8") + zip_bytes + f"\r\n--{boundary}--\r\n".encode("utf-8")

        req = urllib.request.Request(
            f"{API_BASE}/api/problems/{problem_id}/add-tests",
            data=body,
            headers={"Content-Type": f"multipart/form-data; boundary={boundary}"}
        )
        urllib.request.urlopen(req, timeout=10)

    # ----------------- Zakładka 2: Zarządzanie Zadaniami -----------------
    def _build_tab_tasks(self, parent):
        top_bar = tk.Frame(parent, bg="#080b11", highlightthickness=0)
        top_bar.pack(fill="x", pady=(0, 8))

        tk.Label(top_bar, text="Zapisane zadania w Twoim środowisku:", font=("Segoe UI", 10, "bold"), bg="#080b11", fg="#f8fafc", highlightthickness=0).pack(side="left")

        btn_refresh = tk.Button(
            top_bar, text="🔄 Odśwież", font=("Segoe UI", 9), bg="#1e2d4a", fg="#e2e8f0",
            activebackground="#2b3e66", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=10, pady=3, command=self.refresh_tasks_list
        )
        btn_refresh.pack(side="right")

        list_frame = tk.Frame(parent, bg="#111726", highlightthickness=0)
        list_frame.pack(fill="both", expand=True, pady=(0, 10))

        scrollbar = tk.Scrollbar(list_frame, orient="vertical", relief="flat", bd=0, highlightthickness=0)
        self.tasks_listbox = tk.Listbox(
            list_frame, bg="#111726", fg="#f8fafc", selectbackground="#0284c7", selectforeground="#ffffff",
            relief="flat", bd=0, highlightthickness=0, font=("Consolas", 11), yscrollcommand=scrollbar.set, activestyle="none"
        )
        scrollbar.config(command=self.tasks_listbox.yview)
        scrollbar.pack(side="right", fill="y")
        self.tasks_listbox.pack(side="left", fill="both", expand=True, padx=8, pady=8)
        self.tasks_listbox.bind("<<ListboxSelect>>", self._on_task_selected)

        # Panel akcji dla zadania
        self.actions_card = tk.Frame(parent, bg="#111726", padx=12, pady=10, highlightthickness=0)
        self.actions_card.pack(fill="x")

        self.lbl_selected_title = tk.Label(self.actions_card, text="Wybierz zadanie z listy powyżej", font=("Segoe UI", 10, "bold"), bg="#111726", fg="#38bdf8", highlightthickness=0)
        self.lbl_selected_title.pack(anchor="w", pady=(0, 8))

        btn_row = tk.Frame(self.actions_card, bg="#111726", highlightthickness=0)
        btn_row.pack(fill="x")

        self.btn_act_activate = tk.Button(
            btn_row, text="▶ Aktywuj na SD", font=("Segoe UI", 9, "bold"), bg="#0284c7", fg="#ffffff",
            activebackground="#0369a1", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=12, pady=6, command=self._act_activate
        )
        self.btn_act_activate.pack(side="left", padx=(0, 6))

        self.btn_act_vscode = tk.Button(
            btn_row, text="💻 VS Code", font=("Segoe UI", 9), bg="#1e2d4a", fg="#e2e8f0",
            activebackground="#2b3e66", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=12, pady=6, command=self._act_vscode
        )
        self.btn_act_vscode.pack(side="left", padx=(0, 6))

        self.btn_act_add_zip = tk.Button(
            btn_row, text="📦 + Testy (ZIP)", font=("Segoe UI", 9), bg="#1e2d4a", fg="#e2e8f0",
            activebackground="#2b3e66", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=12, pady=6, command=self._act_add_tests_zip
        )
        self.btn_act_add_zip.pack(side="left", padx=(0, 6))

        self.btn_act_delete = tk.Button(
            btn_row, text="🗑️ Usuń", font=("Segoe UI", 9, "bold"), bg="#7f1d1d", fg="#fca5a5",
            activebackground="#991b1b", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=12, pady=6, command=self._act_delete
        )
        self.btn_act_delete.pack(side="right")

    def refresh_tasks_list(self):
        self.tasks_listbox.delete(0, "end")
        self.problems_cache.clear()

        try:
            with urllib.request.urlopen(f"{API_BASE}/api/problems", timeout=2) as res:
                data = json.loads(res.read().decode("utf-8"))
                self.problems_cache = data.get("problems", [])
                self.active_problem_id = (data.get("active") or "").lower()
        except Exception:
            wdir = Path(self.settings.get("workspace_dir", Path.home() / "algodeck-workspace"))
            if wdir.exists():
                for p in sorted(wdir.iterdir()):
                    if p.is_dir() and not p.name.startswith(".") and p.name.lower() != "tests":
                        pid = p.name.lower()
                        mfile = p / ".algo" / "problem.json"
                        t_count = len(list((p / ".algo" / "tests").glob("*.in"))) if (p / ".algo" / "tests").exists() else 0
                        title = pid.upper()
                        if mfile.exists():
                            try:
                                m = json.loads(mfile.read_text(encoding="utf-8"))
                                title = m.get("title", title)
                            except Exception:
                                pass
                        self.problems_cache.append({
                            "problem_id": pid,
                            "title": title,
                            "tests": range(t_count)
                        })

        if not self.problems_cache:
            self.tasks_listbox.insert("end", "  (Brak zadań w workspace. Dodaj nowe w zakładce powyżej)")
            self.lbl_selected_title.config(text="Brak zadań w workspace")
            return

        for prob in self.problems_cache:
            pid = prob.get("problem_id", "").lower()
            title = prob.get("title", pid.upper())
            t_cnt = len(prob.get("tests", []))
            is_act = (pid == self.active_problem_id)
            prefix = "⚡ [AKTYWNE] " if is_act else "  "
            line = f"{prefix}{title} [{pid.upper()}]  •  {t_cnt} testów"
            self.tasks_listbox.insert("end", line)

        self.tasks_listbox.selection_set(0)
        self._on_task_selected()

    def _get_selected_problem(self) -> Optional[Dict[str, Any]]:
        sel = self.tasks_listbox.curselection()
        if not sel: return None
        idx = sel[0]
        if idx < len(self.problems_cache):
            return self.problems_cache[idx]
        return None

    def _on_task_selected(self, event=None):
        prob = self._get_selected_problem()
        if prob:
            pid = prob.get("problem_id", "").upper()
            title = prob.get("title", pid)
            t_cnt = len(prob.get("tests", []))
            self.lbl_selected_title.config(text=f"{title} [{pid}]  •  {t_cnt} zestawów testowych")

    def _act_activate(self):
        prob = self._get_selected_problem()
        if not prob: return
        pid = prob.get("problem_id", "").lower()
        try:
            req = urllib.request.Request(f"{API_BASE}/api/set-active/{pid}", method="POST")
            urllib.request.urlopen(req, timeout=3)
            os.system(f'notify-send "AlgoDeck" "Aktywowano zadanie: {pid.upper()}" 2>/dev/null || true')
            self.refresh_tasks_list()
        except Exception:
            subprocess.run(["bash", "-c", f"$HOME/.local/bin/sd_algo_switch.sh to '{pid}'"])
            self.refresh_tasks_list()

    def _act_vscode(self):
        prob = self._get_selected_problem()
        if not prob: return
        pid = prob.get("problem_id", "").lower()
        wdir = Path(self.settings.get("workspace_dir", Path.home() / "algodeck-workspace")) / pid
        cpp = wdir / f"{pid}.cpp"
        subprocess.Popen(["bash", os.path.expanduser("~/.local/bin/sd_algo_code.sh"), str(wdir), str(cpp)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def _act_add_tests_zip(self):
        prob = self._get_selected_problem()
        if not prob: return
        pid = prob.get("problem_id", "").lower()
        f = filedialog.askopenfilename(
            title=f"Wybierz testy ZIP dla zadania [{pid.upper()}]",
            filetypes=[("Archiwa ZIP", "*.zip"), ("Wszystkie pliki", "*.*")]
        )
        if not f: return
        try:
            self._upload_zip_tests(pid, f)
            messagebox.showinfo("AlgoDeck", f"Pomyślnie dodano nowe testy do zadania {pid.upper()}!")
            self.refresh_tasks_list()
        except Exception as e:
            messagebox.showerror("Błąd", f"Nie udało się zaimportować testów:\n{e}")

    def _act_delete(self):
        prob = self._get_selected_problem()
        if not prob: return
        pid = prob.get("problem_id", "").lower()
        if messagebox.askyesno("AlgoDeck", f"Czy na pewno chcesz bezpowrotnie usunąć zadanie {pid.upper()}?"):
            try:
                req = urllib.request.Request(f"{API_BASE}/api/problem/{pid}", method="DELETE")
                urllib.request.urlopen(req, timeout=3)
            except Exception:
                wdir = Path(self.settings.get("workspace_dir", Path.home() / "algodeck-workspace")) / pid
                shutil.rmtree(wdir, ignore_errors=True)
            self.refresh_tasks_list()

    # ----------------- Zakładka 3: Ustawienia Ogólne -----------------
    def _build_tab_settings(self, parent):
        card = tk.Frame(parent, bg="#111726", padx=16, pady=16, highlightthickness=0)
        card.pack(fill="both", expand=True, pady=(0, 12))

        # Katalog roboczy
        tk.Label(card, text="Główny katalog roboczy zadań (Workspace):", font=("Segoe UI", 9, "bold"), bg="#111726", fg="#94a3b8", highlightthickness=0).pack(anchor="w")
        row_dir = tk.Frame(card, bg="#111726", highlightthickness=0)
        row_dir.pack(fill="x", pady=(2, 16))

        self.entry_sett_dir = tk.Entry(row_dir, bg="#182238", fg="#f8fafc", insertbackground="#38bdf8", relief="flat", bd=0, highlightthickness=0, font=("Consolas", 10))
        self.entry_sett_dir.insert(0, self.settings.get("workspace_dir", str(Path.home() / "algodeck-workspace")))
        self.entry_sett_dir.pack(side="left", fill="x", expand=True, padx=(0, 6), ipady=5)

        btn_browse_dir = tk.Button(
            row_dir, text="Przeglądaj...", font=("Segoe UI", 9), bg="#1e2d4a", fg="#e2e8f0",
            activebackground="#2b3e66", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=12, pady=4, command=self._browse_workspace_dir
        )
        btn_browse_dir.pack(side="left")

        # Tryb Visual Studio Code (Segmentowy przełącznik BEZ BIAŁYCH KÓŁEK)
        tk.Label(card, text="Zachowanie Visual Studio Code:", font=("Segoe UI", 9, "bold"), bg="#111726", fg="#94a3b8", highlightthickness=0).pack(anchor="w")
        self.current_vscode_mode = self.settings.get("vscode_mode", "single_window")

        seg_vsc = tk.Frame(card, bg="#111726", highlightthickness=0)
        seg_vsc.pack(fill="x", pady=(4, 16))

        self.btn_vsc_single = tk.Button(
            seg_vsc, text="📁 Pojedyncze okno (podfoldery w 1 workspace)", font=("Segoe UI", 9, "bold"),
            bg="#0284c7" if self.current_vscode_mode == "single_window" else "#182238",
            fg="#ffffff" if self.current_vscode_mode == "single_window" else "#94a3b8",
            activebackground="#0369a1", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0,
            cursor="hand2", padx=12, pady=6, command=lambda: self._set_vsc_mode("single_window")
        )
        self.btn_vsc_single.pack(side="left", padx=(0, 6))

        self.btn_vsc_sep = tk.Button(
            seg_vsc, text="🪟 Osobne okna dla każdego zadania", font=("Segoe UI", 9, "bold"),
            bg="#0284c7" if self.current_vscode_mode == "separate_windows" else "#182238",
            fg="#ffffff" if self.current_vscode_mode == "separate_windows" else "#94a3b8",
            activebackground="#0369a1", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0,
            cursor="hand2", padx=12, pady=6, command=lambda: self._set_vsc_mode("separate_windows")
        )
        self.btn_vsc_sep.pack(side="left")

        # Powiadomienia (Własny przełącznik BEZ BIAŁYCH CHECKBOXÓW)
        tk.Label(card, text="Powiadomienia systemowe (notify-send):", font=("Segoe UI", 9, "bold"), bg="#111726", fg="#94a3b8", highlightthickness=0).pack(anchor="w")
        self.current_notifications = bool(self.settings.get("notifications", True))

        self.btn_notif_toggle = tk.Button(
            card, text="✔ Powiadomienia włączone" if self.current_notifications else "✖ Powiadomienia wyłączone",
            font=("Segoe UI", 9, "bold"),
            bg="#065f46" if self.current_notifications else "#334155",
            fg="#a7f3d0" if self.current_notifications else "#94a3b8",
            activebackground="#047857", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0,
            cursor="hand2", padx=12, pady=6, command=self._toggle_notifications
        )
        self.btn_notif_toggle.pack(anchor="w", pady=(4, 16))

        # Sekcja Stream Deck
        tk.Label(card, text="Stream Deck (StreamController Flatpak):", font=("Segoe UI", 9, "bold"), bg="#111726", fg="#94a3b8", highlightthickness=0).pack(anchor="w")
        row_sd = tk.Frame(card, bg="#111726", highlightthickness=0)
        row_sd.pack(fill="x", pady=(4, 8))

        btn_sync_sd = tk.Button(
            row_sd, text="🔄 Synchronizuj profile SD", font=("Segoe UI", 9), bg="#1e2d4a", fg="#e2e8f0",
            activebackground="#2b3e66", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=12, pady=6, command=self._sync_streamdeck
        )
        btn_sync_sd.pack(side="left", padx=(0, 8))

        btn_idle_sd = tk.Button(
            row_sd, text="🏠 Pokaż Ekran Główny (ALGO_IDLE)", font=("Segoe UI", 9), bg="#1e2d4a", fg="#e2e8f0",
            activebackground="#2b3e66", activeforeground="#ffffff", relief="flat", bd=0, highlightthickness=0, cursor="hand2", padx=12, pady=6, command=self._show_idle_screen
        )
        btn_idle_sd.pack(side="left")

        # Przycisk zapisu ustawień
        btn_save = tk.Button(
            parent, text="💾 Zapisz Ustawienia", font=("Segoe UI", 11, "bold"),
            bg="#0284c7", fg="#ffffff", activebackground="#0369a1", activeforeground="#ffffff",
            relief="flat", bd=0, highlightthickness=0, cursor="hand2", pady=10, command=self._save_settings
        )
        btn_save.pack(fill="x")

    def _set_vsc_mode(self, mode: str):
        self.current_vscode_mode = mode
        if mode == "single_window":
            self.btn_vsc_single.config(bg="#0284c7", fg="#ffffff")
            self.btn_vsc_sep.config(bg="#182238", fg="#94a3b8")
        else:
            self.btn_vsc_sep.config(bg="#0284c7", fg="#ffffff")
            self.btn_vsc_single.config(bg="#182238", fg="#94a3b8")

    def _toggle_notifications(self):
        self.current_notifications = not self.current_notifications
        if self.current_notifications:
            self.btn_notif_toggle.config(text="✔ Powiadomienia włączone", bg="#065f46", fg="#a7f3d0")
        else:
            self.btn_notif_toggle.config(text="✖ Powiadomienia wyłączone", bg="#334155", fg="#94a3b8")

    def _browse_workspace_dir(self):
        d = filedialog.askdirectory(title="Wybierz katalog roboczy dla zadań")
        if d:
            self.entry_sett_dir.delete(0, "end")
            self.entry_sett_dir.insert(0, d)

    def _sync_streamdeck(self):
        try:
            from backend.streamdeck.streamcontroller_bridge import StreamControllerBridge
            bridge = StreamControllerBridge(Path(self.entry_sett_dir.get().strip()))
            bridge.sync_all_problems()
            messagebox.showinfo("AlgoDeck", "Profile Stream Decka zostały pomyślnie wygenerowane i odświeżone!")
        except Exception as e:
            messagebox.showerror("Błąd", f"Błąd synchronizacji Stream Decka:\n{e}")

    def _show_idle_screen(self):
        subprocess.run(["gdbus", "call", "--session", "--dest", "com.core447.StreamController", "--object-path", "/com/core447/StreamController", "--method", "com.core447.StreamController.ChangePage", "A00SA6042JGA63", "ALGO_IDLE"])

    def _save_settings(self):
        new_settings = {
            "workspace_dir": self.entry_sett_dir.get().strip(),
            "vscode_mode": self.current_vscode_mode,
            "notifications": self.current_notifications
        }
        save_user_settings(new_settings)
        self.settings = new_settings
        messagebox.showinfo("AlgoDeck", "Ustawienia zostały pomyślnie zapisane!")

def main():
    parser = argparse.ArgumentParser(description="AlgoDeck - Panel Kontrolny")
    parser.add_argument("--tab", choices=["new", "tasks", "settings"], default="new", help="Początkowa zakładka")
    parser.add_argument("--pdf", default="", help="Ścieżka do pobranego pliku PDF do automatycznego wczytania")
    args = parser.parse_args()

    root = tk.Tk()
    app = AlgoDeckPanel(root, initial_tab=args.tab, initial_pdf=args.pdf)
    root.mainloop()

if __name__ == "__main__":
    main()
