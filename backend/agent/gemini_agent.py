import json
import logging
import os
import re
from pathlib import Path
from typing import Dict, Any, Optional

from backend.agent.pdf_parser import PDFParser
from backend.agent.prompts import SYSTEM_PROMPT
from backend.config import settings

logger = logging.getLogger("algodeck.agent")

class GeminiAgent:
    def __init__(self, api_key: Optional[str] = None):
        self.api_key = api_key or settings.gemini_api_key or os.getenv("GEMINI_API_KEY", "")
        self.client = None
        self._init_client()

    def _init_client(self):
        """Initializes Google GenAI client if credentials are available."""
        try:
            from google import genai
            if self.api_key:
                self.client = genai.Client(api_key=self.api_key)
            else:
                # Check for Google Application Default Credentials or system account login
                try:
                    self.client = genai.Client()
                except Exception:
                    self.client = None
        except Exception as e:
            logger.warning(f"Could not initialize google-genai client: {e}")
            self.client = None

    def analyze_pdf(self, pdf_path: Path) -> Dict[str, Any]:
        """
        Analyzes a problem statement PDF using Gemini.
        Falls back smoothly to heuristic parsing if API/network is unavailable.
        """
        raw_text = PDFParser.extract_text(pdf_path)
        
        # If Gemini client is ready, query Gemini model
        if self.client:
            try:
                from google.genai import types
                
                logger.info("Sending problem statement to Gemini AI...")
                # We can supply both raw text and PDF bytes for optimal fidelity
                pdf_bytes = PDFParser.extract_bytes(pdf_path)
                
                parts = [
                    types.Part.from_bytes(data=pdf_bytes, mime_type="application/pdf"),
                    types.Part.from_text(text=f"Przeanalizuj to zadanie z Olimpiady Informatycznej / Szkopuła.\n\nTreść tekstowa zadania:\n{raw_text}")
                ]
                
                response = self.client.models.generate_content(
                    model=settings.gemini_model,
                    contents=parts,
                    config=types.GenerateContentConfig(
                        system_instruction=SYSTEM_PROMPT,
                        response_mime_type="application/json"
                    )
                )
                
                response_text = response.text.strip()
                # Clean up any potential markdown formatting
                if response_text.startswith("```json"):
                    response_text = response_text[7:]
                if response_text.endswith("```"):
                    response_text = response_text[:-3]
                response_text = response_text.strip()
                
                parsed_json = json.loads(response_text)
                logger.info(f"Successfully received analysis from Gemini: {parsed_json.get('title')}")
                return parsed_json
            except Exception as e:
                logger.error(f"Gemini AI analysis failed or timed out: {e}. Falling back to heuristic extractor.")
        else:
            logger.info("No Gemini credentials provided. Utilizing high-accuracy heuristic parser.")

        # Heuristic fallback
        return PDFParser.parse_heuristics(raw_text)
