import json
import logging
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Dict, Any, Optional

from backend.agent.pdf_parser import PDFParser
from backend.agent.prompts import SYSTEM_PROMPT
from backend.config import settings

logger = logging.getLogger("algodeck.agent")

class GeminiAgent:
    def __init__(self, api_key: Optional[str] = None):
        self.api_key = api_key or settings.gemini_api_key or os.getenv("GEMINI_API_KEY", "")

    def analyze_pdf(self, pdf_path: Path) -> Dict[str, Any]:
        """
        Analizuje treść PDF:
        1. Jeśli w systemie działa zalogowane CLI (agy lub gemini), używa go bezpośrednio.
        2. Jeśli podano klucz API, używa google-genai SDK.
        3. W przeciwnym razie używa wbudowanego, niezawodnego parsera heurystycznego.
        """
        raw_text = PDFParser.extract_text(pdf_path)

        # 1. Próba: Zalogowane systemowe CLI (tylko gdy użytkownik jawnie włączy USE_AGY_CLI=1)
        if os.getenv("USE_AGY_CLI") == "1":
            agy_bin = shutil.which("agy") or shutil.which("gemini")
            if agy_bin:
                try:
                    logger.info("Wywołuję systemowe CLI agy...")
                    prompt = f"{SYSTEM_PROMPT}\n\nOto treść zadania z PDF:\n{raw_text[:4000]}"
                    proc = subprocess.run(
                        [agy_bin, "-p", prompt],
                        stdin=subprocess.DEVNULL,
                        capture_output=True,
                        text=True,
                        timeout=5
                    )
                    if proc.returncode == 0 and proc.stdout.strip():
                        text = proc.stdout.strip()
                        if text.startswith("```json"):
                            text = text[7:]
                        if text.endswith("```"):
                            text = text[:-3]
                        parsed = json.loads(text.strip())
                        parsed["used_engine"] = "Gemini CLI (agy)"
                        return self._verify_and_compute_tests(parsed)
                except Exception as e:
                    logger.info(f"Błąd CLI agy: {e}")

        # 2. Próba: google-genai z kluczem API
        api_key = self.api_key or settings.gemini_api_key or os.getenv("GEMINI_API_KEY", "")
        if api_key:
            try:
                from google import genai
                from google.genai import types
                client = genai.Client(api_key=api_key)
                logger.info(f"Wysyłam zapytanie do Gemini API ({settings.gemini_model})...")
                response = client.models.generate_content(
                    model=settings.gemini_model,
                    contents=f"Oto pełna treść zadania z pliku PDF:\n\n{raw_text}",
                    config=types.GenerateContentConfig(
                        system_instruction=SYSTEM_PROMPT,
                        response_mime_type="application/json"
                    )
                )
                text = response.text.strip()
                if text.startswith("```json"):
                    text = text[7:]
                if text.endswith("```"):
                    text = text[:-3]
                parsed = json.loads(text.strip())
                parsed["used_engine"] = f"Google Gemini ({settings.gemini_model})"
                parsed = self._verify_and_compute_tests(parsed)
                logger.info(f"Gemini API pomyślnie przeanalizowało zadanie '{parsed.get('title')}' i wygenerowało {len(parsed.get('tests', []))} testów!")
                return parsed
            except Exception as e:
                logger.warning(f"Błąd Gemini API: {e}")

        # 3. Zawsze niezawodny parser heurystyczny
        fallback = PDFParser.parse_heuristics(raw_text)
        fallback["used_engine"] = "Wbudowany parser heurystyczny (lokalny, offline)"
        return fallback

    @staticmethod
    def _verify_and_compute_tests(parsed: Dict[str, Any]) -> Dict[str, Any]:
        """
        Weryfikuje i wylicza poprawne 'expected_output' dla wszystkich testów przy użyciu
        dostarczonego kodu 'python_reference_solution' (Python Oracle).
        Gwarantuje brak halucynacji LLM na wynikach testów.
        """
        py_solution = parsed.get("python_reference_solution")
        tests = parsed.get("tests", [])
        if not tests or not py_solution or not isinstance(py_solution, str):
            return parsed

        py_code = py_solution.strip()
        if not py_code:
            return parsed

        def run_oracle(inp: str) -> Optional[str]:
            try:
                res = subprocess.run(
                    [sys.executable, "-c", py_code],
                    input=inp,
                    capture_output=True,
                    text=True,
                    timeout=3
                )
                if res.returncode == 0:
                    return res.stdout.rstrip("\r\n")
            except Exception as ex:
                logger.debug(f"Błąd uruchomienia wyroczni: {ex}")
            return None

        # 1. Sprawdź, czy wyrocznia przechodzi autentyczne przykłady z treści PDF
        def is_real_official(t: Dict[str, Any]) -> bool:
            nm = t.get("name", "").lower()
            desc = t.get("description", "").lower()
            return ("przykład" in nm or "0a" in nm or "0b" in nm or "0c" in nm or "example" in nm or "sample" in desc) and not t.get("is_edge_case", False)

        real_officials = [t for t in tests if is_real_official(t)]
        if not real_officials:
            real_officials = tests[:1]

        oracle_verified = True
        for ot in real_officials:
            inp = ot.get("input", "")
            exp = ot.get("expected_output", "").strip()
            if not inp or not exp:
                continue
            computed = run_oracle(inp)
            if computed is None:
                oracle_verified = False
                break
            norm_comp = "\n".join(l.rstrip() for l in computed.splitlines())
            norm_exp = "\n".join(l.rstrip() for l in exp.splitlines())
            if norm_comp != norm_exp:
                logger.warning(f"Wyrocznia Pythona nie zgadza się z oficjalnym przykładem '{ot.get('name')}'! Oczekiwano: {norm_exp[:40]}..., wyrocznia dała: {norm_comp[:40]}...")
                oracle_verified = False
                break

        if oracle_verified:
            logger.info(f"✓ Wyrocznia Pythona zweryfikowana z {len(real_officials)} oficjalnymi przykładami! Przeliczam/koryguję testy wygenerowane...")
            for t in tests:
                # Jeśli to nie jest oficjalny przykład z PDF, przelicz wynik komputerową wyrocznią
                if not is_real_official(t):
                    inp = t.get("input", "")
                    if inp:
                        computed = run_oracle(inp)
                        if computed is not None:
                            old_exp = t.get("expected_output", "").strip()
                            if old_exp and old_exp != computed:
                                logger.info(f"Skorygowano halucynację AI dla '{t.get('name')}': zastąpiono błędny wzorzec wyliczeniem wyroczni.")
                            t["expected_output"] = computed
                            t["is_verified"] = True
                            t["is_edge_case"] = True
                            desc = t.get("description", "")
                            t["description"] = f"{desc} (zweryfikowany wyrocznią Python)".strip()
        else:
            logger.warning("Wyrocznia Pythona nie przeszła testów oficjalnych – zachowano oryginalne dane.")

        return parsed
