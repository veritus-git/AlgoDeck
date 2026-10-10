#!/usr/bin/env bash
""":"
# Python wrapper inside bash script
source "$(dirname "$0")/../.venv/bin/activate"
export PYTHONPATH="$(dirname "$0")/.."
exec python3 "$0" "$@"
"""

import sys
import json
import os
from pathlib import Path

from backend.agent.pdf_parser import PDFParser
from backend.workspace.zip_parser import ZipParser
from backend.workspace.builder import WorkspaceBuilder
from backend.runner.executor import TestExecutor
from backend.streamdeck.controller import StreamDeckController
from backend.streamdeck.streamcontroller_bridge import StreamControllerBridge

def main():
    print("=== TEST PIPELINE ALGODECK (100% LOKALNY, KOŁO MAP) ===")
    
    # 1. Test na akc.pdf i akcje_testy.zip
    pdf_path = Path("/home/linux/Pobrane/Downloads/akc.pdf")
    zip_path = Path("/home/linux/Pobrane/Downloads/akcje_testy.zip")
    
    if pdf_path.exists():
        print(f"\n1. Parsowanie PDF: {pdf_path.name}...")
        text = PDFParser.extract_text(pdf_path)
        pdf_info = PDFParser.parse_heuristics(text)
        print(f"   -> Kod: {pdf_info.get('problem_id')}, Tytuł: {pdf_info.get('title')}")
        print(f"   -> Limity: {pdf_info.get('time_limit_sec')}s | {pdf_info.get('memory_limit_mb')}MB")
        print(f"   -> Testy w treści ({len(pdf_info.get('tests', []))}):")
        for t in pdf_info.get('tests', []):
            print(f"      {t.get('name')}: IN='{t.get('input').strip()}' OUT='{t.get('expected_output').strip()}'")

    if zip_path.exists():
        print(f"\n2. Parsowanie ZIP z testami: {zip_path.name}...")
        zip_info = ZipParser.parse_zip(zip_path, pdf_tests=pdf_info.get("tests", []) if pdf_path.exists() else [])
        print(f"   -> Wykryty kod z plików: {zip_info.get('detected_problem_id')}")
        print(f"   -> Liczba testów w paczce: {len(zip_info.get('tests', []))}")
        for t in zip_info.get('tests', [])[:3]:
            print(f"      {t.get('id')}: {t.get('tag')} -> OUT='{t.get('expected_output')}'")

    # 3. Test tworzenia workspace
    print("\n3. Tworzenie środowiska roboczego w test_workspace...")
    test_workspace_dir = Path(__file__).parent.parent / "test_workspace"
    builder = WorkspaceBuilder(base_dir=test_workspace_dir)
    analysis = {
        "problem_id": "akc",
        "title": "Akcje",
        "time_limit_sec": 1.0,
        "memory_limit_mb": 128,
        "tests": zip_info.get("tests", []) if zip_path.exists() else pdf_info.get("tests", [])
    }
    pdir = builder.create_problem_workspace(analysis, original_pdf=pdf_path if pdf_path.exists() else None)
    print(f"   -> Katalog utworzony: {pdir}")
    assert (pdir / "akc.cpp").exists()
    assert (pdir / ".algo" / "problem.json").exists()
    assert (pdir / ".algo" / "test.sh").exists()
    assert (pdir / ".algo" / "run.sh").exists()
    assert (pdir / ".algo" / "kill.sh").exists()

    # 4. Stream Deck
    print("\n4. Test konfiguracji minimalistycznego Stream Decka...")
    executor = TestExecutor(workspace_dir=test_workspace_dir)
    controller = StreamDeckController(executor)
    controller.set_active_problem("akc")
    print(f"   -> Aktywne zadanie: {controller.active_problem_id}")
    for idx in [2, 5, 6, 7, 8, 9]:
        k = controller.keys_state[idx]
        print(f"      [{idx}] {k.get('icon')} {k.get('label'):<8} | status: {k.get('status')}")

    print("\n✅ CAŁY PIPELINE DZIAŁA POPRAWNIE BEZ BŁĘDÓW!")

if __name__ == "__main__":
    main()
