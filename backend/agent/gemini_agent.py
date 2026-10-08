import json
import logging
import os
import shutil
import subprocess
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

        # 1. Próba: Zalogowane systemowe CLI (agy)
        agy_bin = shutil.which("agy") or shutil.which("gemini")
        if agy_bin:
            try:
                logger.info(f"Próba wywołania systemowego CLI: {agy_bin}...")
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
                    parsed["used_engine"] = "Gemini CLI (agy) - zalogowane konto"
                    return parsed
            except Exception as e:
                logger.info(f"CLI agy niedostępne bez interakcji: {e}")

        # 2. Próba: google-genai z kluczem API
        if self.api_key:
            try:
                from google import genai
                from google.genai import types
                client = genai.Client(api_key=self.api_key)
                response = client.models.generate_content(
                    model=settings.gemini_model,
                    contents=raw_text,
                    config=types.GenerateContentConfig(
                        system_instruction=SYSTEM_PROMPT,
                        response_mime_type="application/json"
                    )
                )
                parsed = json.loads(response.text.strip())
                parsed["used_engine"] = "Google GenAI API (klucz API)"
                return parsed
            except Exception as e:
                logger.warning(f"Błąd Gemini API: {e}")

        # 3. Zawsze niezawodny parser heurystyczny
        fallback = PDFParser.parse_heuristics(raw_text)
        fallback["used_engine"] = "Wbudowany parser heurystyczny (lokalny, offline)"
        return fallback
