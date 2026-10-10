#!/usr/bin/env python3
"""
AlgoDeck - Modalne Okno Dodawania Zadania (Koło MAP)
Uruchamiane przyciskiem '+' ze Stream Decka.
Otwiera się dokładnie na środku GŁÓWNEGO (primary) monitora.
"""

import json
import os
import re
import subprocess
import sys
import urllib.request
import urllib.error
import tkinter as tk
from tkinter import messagebox

API_URL = "http://127.0.0.1:8080/api/create-manual"

def get_primary_monitor_geometry():
    """Wykrywa geometrię głównego (primary) monitora z xrandr."""
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

class AddTaskDialog:
    def __init__(self, root: tk.Tk):
        self.root = root
        self.root.title("AlgoDeck — Nowe Zadanie")
        self.root.configure(bg="#0b0e14")
        self.root.resizable(False, False)

        # Wymiary okna
        w = 400
        h = 500

        # Wyśrodkowanie na GŁÓWNYM monitorze
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

        self._build_ui()
        self.entry_code.focus_set()

        # Skróty klawiszowe
        self.root.bind("<Escape>", lambda e: self.root.destroy())

    def _build_ui(self):
        main_frame = tk.Frame(self.root, bg="#0b0e14", padx=16, pady=16)
        main_frame.pack(fill="both", expand=True)

        # Nagłówek
        header_frame = tk.Frame(main_frame, bg="#0b0e14")
        header_frame.pack(fill="x", pady=(0, 12))

        lbl_logo = tk.Label(header_frame, text="⚡", font=("Segoe UI", 16), bg="#0b0e14", fg="#38bdf8")
        lbl_logo.pack(side="left", padx=(0, 8))

        lbl_title = tk.Label(header_frame, text="Nowe Zadanie", font=("Segoe UI", 14, "bold"), bg="#0b0e14", fg="#f8fafc")
        lbl_title.pack(side="left")

        lbl_sub = tk.Label(header_frame, text="KOŁO MAP", font=("Segoe UI", 9, "bold"), bg="#0b0e14", fg="#38bdf8")
        lbl_sub.pack(side="right")

        # Karta formularza
        card = tk.Frame(main_frame, bg="#121722", highlightbackground="#222b3d", highlightthickness=1, padx=12, pady=12)
        card.pack(fill="x", pady=(0, 14))

        # Pole: Kod zadania
        self._label(card, "Nazwa / Kod zadania * (np. chw, kol):")
        self.entry_code = self._entry(card)
        self.entry_code.pack(fill="x", pady=(2, 8))
        self.entry_code.bind("<Return>", lambda e: self.entry_title.focus_set())

        # Pole: Pełny tytuł
        self._label(card, "Pełny tytuł (opcjonalny):")
        self.entry_title = self._entry(card)
        self.entry_title.pack(fill="x", pady=(2, 8))
        self.entry_title.bind("<Return>", lambda e: self._submit())

        # Rząd: Limity (Idealnie po 50% szerokości)
        row_limits = tk.Frame(card, bg="#121722")
        row_limits.pack(fill="x", pady=(0, 8))
        row_limits.columnconfigure(0, weight=1, uniform="lim")
        row_limits.columnconfigure(1, weight=1, uniform="lim")

        col_time = tk.Frame(row_limits, bg="#121722")
        col_time.grid(row=0, column=0, sticky="ew", padx=(0, 5))
        self._label(col_time, "Czas (s):")
        self.entry_time = self._entry(col_time)
        self.entry_time.insert(0, "1.0")
        self.entry_time.pack(fill="x", pady=(2, 0))

        col_mem = tk.Frame(row_limits, bg="#121722")
        col_mem.grid(row=0, column=1, sticky="ew", padx=(5, 0))
        self._label(col_mem, "RAM (MB):")
        self.entry_mem = self._entry(col_mem)
        self.entry_mem.insert(0, "128")
        self.entry_mem.pack(fill="x", pady=(2, 0))

        # Test opcjonalny
        self._label(card, "Przykładowe wejście cin (opcjonalne):")
        self.text_in = tk.Text(card, height=2, bg="#181f2f", fg="#f8fafc", insertbackground="#38bdf8",
                               relief="flat", highlightbackground="#222b3d", highlightthickness=1, font=("Consolas", 10))
        self.text_in.pack(fill="x", pady=(2, 8))

        self._label(card, "Oczekiwane wyjście cout (opcjonalne):")
        self.text_out = tk.Text(card, height=2, bg="#181f2f", fg="#f8fafc", insertbackground="#38bdf8",
                                relief="flat", highlightbackground="#222b3d", highlightthickness=1, font=("Consolas", 10))
        self.text_out.pack(fill="x", pady=(2, 4))

        # Przyciski dolne (BEZ białych ramek obrysu)
        btn_frame = tk.Frame(main_frame, bg="#0b0e14")
        btn_frame.pack(fill="x")

        self.btn_create = tk.Button(
            btn_frame, text="Utwórz zadanie", font=("Segoe UI", 11, "bold"),
            bg="#0284c7", fg="#ffffff", activebackground="#0369a1", activeforeground="#ffffff",
            relief="flat", bd=0, highlightthickness=0, cursor="hand2", pady=8, command=self._submit
        )
        self.btn_create.pack(side="left", fill="x", expand=True, padx=(0, 6))

        btn_cancel = tk.Button(
            btn_frame, text="Anuluj", font=("Segoe UI", 11),
            bg="#181f2f", fg="#94a3b8", activebackground="#222b3d", activeforeground="#f8fafc",
            relief="flat", bd=0, highlightthickness=0, cursor="hand2", pady=8, command=self.root.destroy
        )
        btn_cancel.pack(side="left", padx=(6, 0), ipadx=14)

    def _label(self, parent, text):
        lbl = tk.Label(parent, text=text, font=("Segoe UI", 9, "bold"), bg="#121722", fg="#94a3b8", anchor="w")
        lbl.pack(fill="x")
        return lbl

    def _entry(self, parent):
        return tk.Entry(parent, bg="#181f2f", fg="#f8fafc", insertbackground="#38bdf8",
                        relief="flat", highlightbackground="#222b3d", highlightthickness=1, font=("Consolas", 11))

    def _submit(self):
        code = self.entry_code.get().strip().lower()
        if not code:
            messagebox.showwarning("AlgoDeck", "Podaj kod/nazwę zadania!")
            self.entry_code.focus_set()
            return

        title = self.entry_title.get().strip() or code.capitalize()
        try:
            time_lim = float(self.entry_time.get().strip())
        except ValueError:
            time_lim = 1.0

        try:
            mem_lim = int(self.entry_mem.get().strip())
        except ValueError:
            mem_lim = 128

        tests = []
        inp = self.text_in.get("1.0", "end-1c").strip()
        out = self.text_out.get("1.0", "end-1c").strip()
        if inp or out:
            tests.append({
                "id": "test_1",
                "name": "Przykład 1",
                "input": inp,
                "expected_output": out
            })

        payload = {
            "problem_id": code,
            "title": title,
            "time_limit": time_lim,
            "memory_limit": mem_lim,
            "tests": tests
        }

        self.btn_create.config(text="Tworzenie...", state="disabled")
        self.root.update()

        try:
            data = json.dumps(payload).encode("utf-8")
            req = urllib.request.Request(API_URL, data=data, headers={"Content-Type": "application/json"})
            with urllib.request.urlopen(req, timeout=5) as res:
                if res.status in (200, 201):
                    os.system(f'notify-send "AlgoDeck" "Utworzono zadanie {code.upper()}! Otwarto VS Code." 2>/dev/null || true')
                    self.root.destroy()
                    return
        except Exception as e:
            messagebox.showerror("Błąd", f"Nie udało się połączyć z AlgoDeck:\n{e}")
            self.btn_create.config(text="Utwórz zadanie", state="normal")

def main():
    root = tk.Tk()
    app = AddTaskDialog(root)
    root.mainloop()

if __name__ == "__main__":
    main()
