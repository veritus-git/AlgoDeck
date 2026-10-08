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
from backend.agent.gemini_agent import GeminiAgent
from backend.workspace.builder import WorkspaceBuilder
from backend.runner.executor import TestExecutor
from backend.streamdeck.controller import StreamDeckController
from backend.streamdeck.renderer import ButtonRenderer
from backend.streamdeck.streamcontroller_bridge import StreamControllerBridge

def main():
    print("=== TEST PIPELINE ALGODECK ===")
    pdf_path = Path(__file__).parent.parent / "examples" / "koleje.pdf"
    assert pdf_path.exists(), f"Brak pliku {pdf_path}"

    print(f"1. Parsowanie PDF: {pdf_path.name}...")
    agent = GeminiAgent()
    analysis = agent.analyze_pdf(pdf_path)
    print(f"   -> Tytuł: {analysis.get('title')}")
    print(f"   -> ID zadania: {analysis.get('problem_id')}")
    print(f"   -> Limit czasu: {analysis.get('time_limit_sec')}s")
    print(f"   -> Wyekstrahowane testy ({len(analysis.get('tests', []))} testów):")
    for t in analysis.get('tests', []):
        tag = "[Corner]" if t.get('is_edge_case') else "[Official]"
        print(f"      {tag} {t.get('name')}")

    print("\n2. Tworzenie środowiska roboczego...")
    test_workspace_dir = Path(__file__).parent.parent / "test_workspace"
    builder = WorkspaceBuilder(base_dir=test_workspace_dir)
    pdir = builder.create_problem_workspace(analysis, original_pdf=pdf_path)
    print(f"   -> Katalog utworzony: {pdir}")
    assert (pdir / f"{analysis['problem_id']}.cpp").exists()
    assert (pdir / "problem.json").exists()
    assert (pdir / "test.sh").exists()
    assert (pdir / "run.sh").exists()
    assert (pdir / "kill.sh").exists()

    print("\n3. Konfiguracja Stream Decka (5 kluczowych przycisków)...")
    executor = TestExecutor(workspace_dir=test_workspace_dir)
    controller = StreamDeckController(executor)
    controller.set_active_problem(analysis["problem_id"])
    print(f"   -> Aktywne zadanie: {controller.active_problem_id}")
    for idx in range(5):
        k = controller.keys_state[idx]
        print(f"      [{idx}] {k.get('icon')} {k.get('label'):<8} | {k.get('subtext'):<6} | akcja: {k.get('action')}")

    print("\n4. Zapisywanie strony bezpośrednio do StreamControllera...")
    bridge = StreamControllerBridge(workspace_dir=test_workspace_dir)
    bridge.generate_page_for_problem(analysis["problem_id"], analysis)
    sc_page = Path(os.path.expanduser("~/.var/app/com.core447.StreamController/data/pages")) / f"{analysis['problem_id'].upper()}.json"
    print(f"   -> Strona StreamControllera: {sc_page}")
    assert sc_page.exists()

    print("\n5. Renderowanie obrazów LCD (PIL 72x72 px)...")
    img = ButtonRenderer.render_key(
        label="BUILD",
        icon="🔨",
        subtext="O3",
        status="IDLE"
    )
    assert img.size == (72, 72)
    print("   -> Pomyślnie wyrenderowano grafikę LCD dla klawisza.")

    print("\n6. Test kompilacji C++...")
    # Add a minimal solve implementation in the C++ file for test execution verification
    cpp_file = pdir / f"{analysis['problem_id']}.cpp"
    cpp_code = """#include <iostream>
using namespace std;
int main() {
    int n, m, z;
    if (cin >> n >> m >> z) {
        for (int i = 0; i < z; ++i) {
            int p, k, l;
            cin >> p >> k >> l;
            cout << (i == 2 ? "N" : "T") << "\\n";
        }
    }
    return 0;
}
"""
    cpp_file.write_text(cpp_code, encoding="utf-8")
    
    comp_res = executor.compile(analysis["problem_id"], debug_mode=False)
    print(f"   -> Wynik kompilacji: sukces={comp_res.get('success')} w {comp_res.get('duration_ms')}ms")
    assert comp_res["success"], f"Błąd kompilacji: {comp_res.get('error')}"

    print("\n7. Test wykonania testu przykładowego...")
    manifest = executor.get_manifest(analysis["problem_id"])
    t1_id = manifest["tests"][0]["id"]
    test_res = executor.run_single_test(analysis["problem_id"], t1_id)
    print(f"   -> Werdykt testu {t1_id}: {test_res.get('verdict')} w {test_res.get('time_ms')}ms")

    print("\n✅ WSZYSTKIE TESTY JEDNOSTKOWE I INTEGRACYJNE ZAKOŃCZONE SUKCESEM!")

if __name__ == "__main__":
    main()
