import asyncio
import io
import json
import logging
import os
import re
import shutil
from pathlib import Path
from typing import Dict, Any, List, Optional

from fastapi import FastAPI, UploadFile, File, Form, WebSocket, WebSocketDisconnect, HTTPException, Body
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import HTMLResponse, JSONResponse, FileResponse
from fastapi.staticfiles import StaticFiles

from backend.agent.pdf_parser import PDFParser
from backend.config import settings
from backend.runner.executor import TestExecutor
from backend.streamdeck.controller import StreamDeckController
from backend.streamdeck.hardware import StreamDeckHardwareDriver
from backend.streamdeck.streamcontroller_bridge import StreamControllerBridge
from backend.workspace.builder import WorkspaceBuilder
from backend.workspace.zip_parser import ZipParser

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(name)s: %(message)s")
logger = logging.getLogger("algodeck.server")

app = FastAPI(title="AlgoDeck ⚡ Koło MAP", version="2.0.0")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Główne podsystemy (100% lokalne, zero AI)
executor = TestExecutor()
workspace_builder = WorkspaceBuilder()
streamdeck_controller = StreamDeckController(executor)
streamcontroller_bridge = StreamControllerBridge()
hardware_driver = StreamDeckHardwareDriver(streamdeck_controller)

# WebSocket connection manager
class ConnectionManager:
    def __init__(self):
        self.active_connections: List[WebSocket] = []

    async def connect(self, websocket: WebSocket):
        await websocket.accept()
        self.active_connections.append(websocket)

    def disconnect(self, websocket: WebSocket):
        if websocket in self.active_connections:
            self.active_connections.remove(websocket)

    async def broadcast(self, message: Dict[str, Any]):
        for connection in list(self.active_connections):
            try:
                await connection.send_json(message)
            except Exception:
                self.disconnect(connection)

ws_manager = ConnectionManager()

def on_streamdeck_keys_changed(keys_state: List[Dict[str, Any]]):
    try:
        loop = asyncio.get_event_loop()
        if loop.is_running():
            asyncio.create_task(ws_manager.broadcast({
                "type": "STREAMDECK_UPDATED",
                "keys": keys_state,
                "active_problem": streamdeck_controller.active_problem_id
            }))
    except Exception as e:
        logger.debug(f"Broadcast warning: {e}")

streamdeck_controller.add_listener(on_streamdeck_keys_changed)

@app.on_event("startup")
async def startup_event():
    logger.info("Uruchamianie AlgoDeck v2.0.0 (Koło MAP)...")
    loop = asyncio.get_running_loop()
    hardware_driver.start(loop)
    # Wygeneruj bazowe geometryczne ikony i zsynchronizuj menu oraz profile zadań
    try:
        streamcontroller_bridge.ensure_vector_icons()
        streamcontroller_bridge.sync_all_problems()
    except Exception as e:
        logger.warning(f"Ostrzeżenie startowe Stream Deck: {e}")

@app.on_event("shutdown")
def shutdown_event():
    hardware_driver.close()

# ----------------- Status & Health -----------------

@app.get("/api/status")
def get_status():
    return {
        "status": "ok",
        "version": "2.0.0",
        "active_problem": streamdeck_controller.active_problem_id,
        "workspace_dir": str(settings.workspace_dir),
        "hardware_connected": hardware_driver.is_connected
    }

# ----------------- Poczekalnia i Trwałość (Staging PDF + ZIP) -----------------

STAGING_DIR = Path("/tmp/algodeck_staging")

def _compute_staged_state() -> Dict[str, Any]:
    STAGING_DIR.mkdir(parents=True, exist_ok=True)
    pdf_path = STAGING_DIR / "staged_pdf.pdf"
    zip_path = STAGING_DIR / "staged_zip.zip"
    meta_path = STAGING_DIR / "meta.json"

    meta: Dict[str, Any] = {}
    if meta_path.exists():
        try:
            meta = json.loads(meta_path.read_text(encoding="utf-8"))
        except Exception:
            meta = {}

    has_pdf = pdf_path.exists()
    has_zip = zip_path.exists()

    if not has_pdf and not has_zip:
        meta_path.unlink(missing_ok=True)
        return {
            "has_pdf": False,
            "has_zip": False,
            "problem_id": "",
            "title": "",
            "time_limit_sec": 1.0,
            "memory_limit_mb": 128,
            "tests_count": 0,
            "tests": []
        }

    pdf_info: Dict[str, Any] = {}
    if has_pdf:
        try:
            text = PDFParser.extract_text(pdf_path)
            pdf_info = PDFParser.parse_heuristics(text)
        except Exception as e:
            logger.warning(f"Błąd parsowania staged PDF: {e}")

    zip_info: Dict[str, Any] = {}
    if has_zip:
        try:
            zip_info = ZipParser.parse_zip(zip_path, pdf_tests=pdf_info.get("tests", []))
        except Exception as e:
            logger.warning(f"Błąd parsowania staged ZIP: {e}")

    final_id = (
        meta.get("problem_id") or
        zip_info.get("detected_problem_id") or
        pdf_info.get("problem_id") or
        "zad"
    ).lower().strip()
    final_id = re.sub(r'[^a-zA-Z0-9_-]', '', final_id) or "zad"

    final_title = (meta.get("title") or pdf_info.get("title") or final_id.upper()).strip()
    final_time = meta.get("time_limit_sec") or pdf_info.get("time_limit_sec", 1.0)
    final_mem = meta.get("memory_limit_mb") or pdf_info.get("memory_limit_mb", 128)

    tests = zip_info.get("tests") or pdf_info.get("tests") or []
    tests_count = len(tests)

    result = {
        "has_pdf": has_pdf,
        "pdf_filename": meta.get("pdf_filename", "zadanie.pdf"),
        "pdf_size": pdf_path.stat().st_size if has_pdf else 0,
        "has_zip": has_zip,
        "zip_filename": meta.get("zip_filename", "testy.zip"),
        "zip_size": zip_path.stat().st_size if has_zip else 0,
        "problem_id": final_id,
        "title": final_title,
        "time_limit_sec": final_time,
        "memory_limit_mb": final_mem,
        "tests_count": tests_count,
        "tests": tests
    }

    try:
        meta_path.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    except Exception:
        pass

    return result

@app.get("/api/staged-state")
def get_staged_state():
    """Zwraca stan aktualnie wgranych plików (PDF/ZIP) w poczekalni (staging)."""
    return _compute_staged_state()

@app.post("/api/stage-file")
async def stage_file(
    file: UploadFile = File(...),
    file_type: str = Form(...)  # "pdf" | "zip"
):
    """Zapisuje plik PDF lub ZIP w trwałej poczekalni i automatycznie parsuje metadane."""
    STAGING_DIR.mkdir(parents=True, exist_ok=True)
    meta_path = STAGING_DIR / "meta.json"
    meta = {}
    if meta_path.exists():
        try:
            meta = json.loads(meta_path.read_text(encoding="utf-8"))
        except Exception:
            pass

    clean_type = file_type.lower().strip()
    if clean_type == "pdf":
        target = STAGING_DIR / "staged_pdf.pdf"
        with open(target, "wb") as buffer:
            shutil.copyfileobj(file.file, buffer)
        meta["pdf_filename"] = file.filename
    elif clean_type == "zip":
        target = STAGING_DIR / "staged_zip.zip"
        with open(target, "wb") as buffer:
            shutil.copyfileobj(file.file, buffer)
        meta["zip_filename"] = file.filename
    else:
        raise HTTPException(status_code=400, detail="Nieobsługiwany typ pliku (wymagany pdf lub zip).")

    meta_path.write_text(json.dumps(meta, ensure_ascii=False, indent=2), encoding="utf-8")
    return _compute_staged_state()

@app.post("/api/clear-staged")
def clear_staged(payload: Dict[str, Any] = Body(default={})):
    """Usuwa plik PDF, ZIP lub oba z poczekalni."""
    ft = payload.get("file_type", "all").lower()
    pdf_path = STAGING_DIR / "staged_pdf.pdf"
    zip_path = STAGING_DIR / "staged_zip.zip"
    meta_path = STAGING_DIR / "meta.json"

    meta = {}
    if meta_path.exists():
        try:
            meta = json.loads(meta_path.read_text(encoding="utf-8"))
        except Exception:
            pass

    if ft in ("pdf", "all"):
        pdf_path.unlink(missing_ok=True)
        meta.pop("pdf_filename", None)
    if ft in ("zip", "all"):
        zip_path.unlink(missing_ok=True)
        meta.pop("zip_filename", None)

    if not pdf_path.exists() and not zip_path.exists():
        meta_path.unlink(missing_ok=True)
    else:
        meta_path.write_text(json.dumps(meta, ensure_ascii=False, indent=2), encoding="utf-8")

    return _compute_staged_state()

@app.post("/api/start-staged")
async def start_staged(payload: Dict[str, Any] = Body(default={})):
    """Rozpoczyna zadanie na bazie plików w poczekalni (PDF i/lub ZIP)."""
    state = _compute_staged_state()
    if not state.get("has_pdf") and not state.get("has_zip"):
        raise HTTPException(status_code=400, detail="Brak wgranych plików (PDF lub ZIP) w poczekalni.")

    pid = (payload.get("problem_id") or state.get("problem_id") or "zad").lower().strip()
    pid = re.sub(r'[^a-zA-Z0-9_-]', '', pid) or "zad"

    title = (payload.get("title") or state.get("title") or pid.upper()).strip()
    time_limit = float(payload.get("time_limit") or state.get("time_limit_sec") or 1.0)
    memory_limit = int(payload.get("memory_limit") or state.get("memory_limit_mb") or 128)

    tests = state.get("tests", [])

    analysis = {
        "problem_id": pid,
        "title": title,
        "time_limit_sec": time_limit,
        "memory_limit_mb": memory_limit,
        "tests": tests
    }

    pdf_path = STAGING_DIR / "staged_pdf.pdf" if state.get("has_pdf") else None

    # Tworzenie workspace
    await ws_manager.broadcast({"type": "PIPELINE_STEP", "step": "WORKSPACE", "message": f"Tworzenie workspace dla '{pid}'..."})
    problem_dir = workspace_builder.create_problem_workspace(analysis, original_pdf=pdf_path)

    # Konfiguracja Stream Decka
    streamdeck_controller.set_active_problem(pid)
    streamcontroller_bridge.generate_page_for_problem(pid, analysis, switch_now=True)

    # Wyczyść staging
    clear_staged({"file_type": "all"})

    await ws_manager.broadcast({
        "type": "PIPELINE_STEP",
        "step": "COMPLETE",
        "message": f"Środowisko gotowe! VS Code uruchomiony dla {pid}.",
        "problem_id": pid
    })

    return {
        "success": True,
        "problem_id": pid,
        "title": title,
        "tests_count": len(tests),
        "workspace_path": str(problem_dir.resolve()),
        "keys": streamdeck_controller.keys_state
    }

# ----------------- Import Zadania (PDF + ZIP) -----------------

@app.post("/api/preview-import")
async def preview_import(
    pdf: Optional[UploadFile] = File(None),
    zip_file: Optional[UploadFile] = File(None, alias="zip"),
    folder_path: Optional[str] = Form(None)
):
    """Szybki podgląd parametrów zadania i liczby testów przed utworzeniem workspace."""
    pdf_data: Dict[str, Any] = {}
    zip_data: Dict[str, Any] = {}

    temp_dir = Path("/tmp/algodeck_previews")
    temp_dir.mkdir(parents=True, exist_ok=True)

    if pdf and pdf.filename:
        temp_pdf = temp_dir / pdf.filename
        with open(temp_pdf, "wb") as buffer:
            shutil.copyfileobj(pdf.file, buffer)
        text = PDFParser.extract_text(temp_pdf)
        pdf_data = PDFParser.parse_heuristics(text)
        temp_pdf.unlink(missing_ok=True)

    if zip_file and zip_file.filename:
        temp_zip = temp_dir / zip_file.filename
        with open(temp_zip, "wb") as buffer:
            shutil.copyfileobj(zip_file.file, buffer)
        zip_data = ZipParser.parse_zip(temp_zip, pdf_tests=pdf_data.get("tests", []))
        temp_zip.unlink(missing_ok=True)
    elif folder_path and Path(folder_path).is_dir():
        zip_data = ZipParser.parse_directory(Path(folder_path), pdf_tests=pdf_data.get("tests", []))

    problem_id = zip_data.get("detected_problem_id") or pdf_data.get("problem_id") or "zad"
    title = pdf_data.get("title") or problem_id.upper()
    time_limit = pdf_data.get("time_limit_sec", 1.0)
    memory_limit = pdf_data.get("memory_limit_mb", 128)

    tests_count = len(zip_data.get("tests", [])) if zip_data.get("tests") else len(pdf_data.get("tests", []))

    return {
        "problem_id": problem_id,
        "title": title,
        "time_limit_sec": time_limit,
        "memory_limit_mb": memory_limit,
        "tests_count": tests_count,
        "has_pdf": bool(pdf_data),
        "has_zip": bool(zip_data or folder_path)
    }

@app.post("/api/import-task")
async def import_task(
    pdf: Optional[UploadFile] = File(None),
    zip_file: Optional[UploadFile] = File(None, alias="zip"),
    folder_path: Optional[str] = Form(None),
    problem_id: Optional[str] = Form(None),
    title: Optional[str] = Form(None),
    time_limit: Optional[float] = Form(None),
    memory_limit: Optional[int] = Form(None)
):
    """
    Importuje zadanie na podstawie pliku PDF, archiwum ZIP lub lokalnego folderu z testami.
    W 100% deterministyczny (bez AI), błyskawicznie tworzy katalog, testy, konfiguruje Stream Deck i odpala VS Code.
    """
    temp_dir = Path("/tmp/algodeck_uploads")
    temp_dir.mkdir(parents=True, exist_ok=True)

    saved_pdf_path: Optional[Path] = None
    pdf_info: Dict[str, Any] = {}
    zip_info: Dict[str, Any] = {}

    # 1. Parsowanie PDF
    if pdf and pdf.filename:
        saved_pdf_path = temp_dir / pdf.filename
        with open(saved_pdf_path, "wb") as buffer:
            shutil.copyfileobj(pdf.file, buffer)
        await ws_manager.broadcast({"type": "PIPELINE_STEP", "step": "PARSING_PDF", "message": "Analiza treści PDF..."})
        text = PDFParser.extract_text(saved_pdf_path)
        pdf_info = PDFParser.parse_heuristics(text)

    # 2. Parsowanie ZIP lub folderu z testami
    if zip_file and zip_file.filename:
        temp_zip = temp_dir / zip_file.filename
        with open(temp_zip, "wb") as buffer:
            shutil.copyfileobj(zip_file.file, buffer)
        await ws_manager.broadcast({"type": "PIPELINE_STEP", "step": "PARSING_ZIP", "message": "Rozpakowywanie testów z ZIP..."})
        zip_info = ZipParser.parse_zip(temp_zip, pdf_tests=pdf_info.get("tests", []))
        temp_zip.unlink(missing_ok=True)
    elif folder_path and Path(folder_path).is_dir():
        await ws_manager.broadcast({"type": "PIPELINE_STEP", "step": "PARSING_DIR", "message": f"Ładowanie testów z katalogu {folder_path}..."})
        zip_info = ZipParser.parse_directory(Path(folder_path), pdf_tests=pdf_info.get("tests", []))

    # 3. Scalanie parametrów
    final_id = (
        problem_id or
        zip_info.get("detected_problem_id") or
        pdf_info.get("problem_id") or
        "zad"
    ).lower().strip()
    final_id = re.sub(r'[^a-zA-Z0-9_-]', '', final_id) or "zad"

    final_title = (title or pdf_info.get("title") or final_id.upper()).strip()
    final_time = time_limit or pdf_info.get("time_limit_sec", 1.0)
    final_mem = memory_limit or pdf_info.get("memory_limit_mb", 128)

    # Zestaw testów: z ZIP (priorytet) lub z PDF
    tests: List[Dict[str, Any]] = []
    if zip_info.get("tests"):
        tests = zip_info["tests"]
        # Jeśli PDF miał dodatkowe testy, które nie były w ZIP, dołącz je
        existing_names = {t["id"].lower() for t in tests}
        for pt in pdf_info.get("tests", []):
            if pt["id"].lower() not in existing_names:
                tests.append(pt)
    elif pdf_info.get("tests"):
        tests = pdf_info["tests"]

    analysis = {
        "problem_id": final_id,
        "title": final_title,
        "time_limit_sec": final_time,
        "memory_limit_mb": final_mem,
        "tests": tests
    }

    # 4. Tworzenie katalogu roboczego i C++
    await ws_manager.broadcast({"type": "PIPELINE_STEP", "step": "WORKSPACE", "message": f"Tworzenie workspace dla '{final_id}'..."})
    problem_dir = workspace_builder.create_problem_workspace(analysis, original_pdf=saved_pdf_path)

    # 5. Konfiguracja Stream Decka
    streamdeck_controller.set_active_problem(final_id)
    streamcontroller_bridge.sync_all_problems()
    streamcontroller_bridge.switch_to_page(final_id)

    await ws_manager.broadcast({
        "type": "PIPELINE_STEP",
        "step": "COMPLETE",
        "message": "Środowisko gotowe! VS Code uruchomiony.",
        "problem_id": final_id
    })

    return {
        "success": True,
        "problem_id": final_id,
        "title": final_title,
        "tests_count": len(tests),
        "workspace_path": str(problem_dir.resolve()),
        "keys": streamdeck_controller.keys_state
    }

@app.post("/api/upload-pdf")
async def upload_pdf_legacy(file: UploadFile = File(...)):
    """Wsteczna kompatybilność z poprzednim endpointem."""
    return await import_task(pdf=file)

# ----------------- Ręczny Workspace -----------------

@app.post("/api/create-manual")
async def create_manual(payload: Dict[str, Any] = Body(...)):
    """
    Ręczne tworzenie zadania (np. dla prostych zadań lub szybkiego prototypowania).
    Wymagane jest tylko pole problem_id.
    """
    raw_id = payload.get("problem_id", "").strip()
    if not raw_id:
        raise HTTPException(status_code=400, detail="Pole 'problem_id' (nazwa/kod zadania) jest wymagane.")

    problem_id = re.sub(r'[^a-zA-Z0-9_-]', '', raw_id.lower())
    if not problem_id:
        raise HTTPException(status_code=400, detail="Nieprawidłowa nazwa zadania (dozwolone litery, cyfry, myślnik, podkreślenie).")

    title = payload.get("title", "").strip() or problem_id.capitalize()
    time_limit = float(payload.get("time_limit", 1.0))
    memory_limit = int(payload.get("memory_limit", 128))

    raw_tests = payload.get("tests", [])
    tests: List[Dict[str, Any]] = []

    for i, t in enumerate(raw_tests, 1):
        inp = t.get("input", "")
        out = t.get("expected_output", "")
        if inp.strip() or out.strip():
            tests.append({
                "id": t.get("id") or f"test_{i}",
                "name": t.get("name") or f"Test {i}",
                "input": inp,
                "expected_output": out,
                "tag": "[RĘCZNY TEST]"
            })

    analysis = {
        "problem_id": problem_id,
        "title": title,
        "time_limit_sec": time_limit,
        "memory_limit_mb": memory_limit,
        "tests": tests
    }

    problem_dir = workspace_builder.create_problem_workspace(analysis)
    streamdeck_controller.set_active_problem(problem_id)
    streamcontroller_bridge.sync_all_problems()
    streamcontroller_bridge.switch_to_page(problem_id)

    await ws_manager.broadcast({
        "type": "PIPELINE_STEP",
        "step": "COMPLETE",
        "message": f"Utworzono zadanie {problem_id}!",
        "problem_id": problem_id
    })

    return {
        "success": True,
        "problem_id": problem_id,
        "title": title,
        "tests_count": len(tests),
        "workspace_path": str(problem_dir.resolve()),
        "keys": streamdeck_controller.keys_state
    }

# ----------------- Galeria i Zarządzanie Zadaniami -----------------

@app.get("/api/problems")
def list_problems():
    """Zwraca listę wszystkich zapisanych zadań z metadanymi."""
    wdir = settings.workspace_dir
    problems = []
    if wdir.exists():
        for p in sorted(wdir.iterdir()):
            if not p.is_dir() or p.name.startswith("."):
                continue
            pid = p.name.lower()
            if pid in ("tests",):
                continue
            mfile = p / ".algo" / "problem.json"
            if not mfile.exists():
                mfile = p / "problem.json"
            if mfile.exists():
                try:
                    manifest = json.loads(mfile.read_text(encoding="utf-8"))
                    problems.append(manifest)
                except Exception:
                    pass
            elif (p / f"{pid}.cpp").exists():
                # Workspace bez manifestu
                problems.append({
                    "problem_id": pid,
                    "title": pid.upper(),
                    "tests": [],
                    "workspace_path": str(p.resolve())
                })
    return {"problems": problems, "active": streamdeck_controller.active_problem_id}

@app.get("/api/problem/{problem_id}")
def get_problem(problem_id: str):
    pid = problem_id.lower()
    manifest = executor.get_manifest(pid)
    pdir = executor.get_problem_dir(pid)
    src_file = pdir / f"{pid}.cpp"
    source_code = src_file.read_text(encoding="utf-8") if src_file.exists() else ""

    return {
        "manifest": manifest or {"problem_id": pid, "title": pid.upper()},
        "source_code": source_code,
        "is_active": streamdeck_controller.active_problem_id == pid
    }

@app.delete("/api/problem/{problem_id}")
def delete_problem(problem_id: str):
    """Usuwa zadanie z dysku, czyści jego stronę ze StreamControllera i aktualizuje menu."""
    pid = problem_id.lower().strip()
    pdir = settings.workspace_dir / pid
    if not pdir.exists():
        raise HTTPException(status_code=404, detail=f"Zadanie '{pid}' nie istnieje w workspace.")

    # Usuń katalog roboczy zadania
    shutil.rmtree(pdir, ignore_errors=True)

    # Usuń ze StreamControllera
    streamcontroller_bridge.remove_problem(pid)

    # Jeśli usunięto aktywne zadanie, przełącz na inne lub wyczyść
    if streamdeck_controller.active_problem_id == pid:
        remaining = [
            p.name.lower() for p in settings.workspace_dir.iterdir()
            if p.is_dir() and not p.name.startswith(".") and p.name != "tests"
        ]
        if remaining:
            new_active = remaining[0]
            streamdeck_controller.set_active_problem(new_active)
            streamcontroller_bridge.switch_to_page(new_active)
        else:
            streamdeck_controller.set_active_problem("")
            streamcontroller_bridge.switch_to_page("ALGO_IDLE")

    return {"success": True, "deleted": pid}

@app.post("/api/problems/{problem_id}/add-tests")
async def add_tests_to_problem(
    problem_id: str,
    file: UploadFile = File(...)
):
    """
    Dodaje zestaw testów (z pliku ZIP) do istniejącego zadania w workspace.
    Aktualizuje .algo/tests/, problem.json oraz regeneruje .algo/test.sh.
    """
    pid = problem_id.lower().strip()
    pdir = settings.workspace_dir / pid
    if not pdir.exists() or not pdir.is_dir():
        raise HTTPException(status_code=404, detail=f"Zadanie '{pid}' nie istnieje w workspace.")

    algo_dir = pdir / ".algo"
    tests_dir = algo_dir / "tests"
    tests_dir.mkdir(parents=True, exist_ok=True)

    content = await file.read()
    if not content:
        raise HTTPException(status_code=400, detail="Plik ZIP jest pusty.")

    try:
        parsed = ZipParser.parse_zip(io.BytesIO(content))
    except Exception as e:
        logger.error(f"Błąd rozpakowywania ZIP dla {pid}: {e}")
        raise HTTPException(status_code=400, detail=f"Błąd czytania archiwum ZIP: {e}")

    new_tests = parsed.get("tests", [])
    if not new_tests:
        raise HTTPException(status_code=400, detail="Nie znaleziono testów (.in / .out) w pliku ZIP.")

    # Wczytaj istniejący manifest problem.json
    manifest_path = algo_dir / "problem.json"
    manifest: Dict[str, Any] = {}
    if manifest_path.exists():
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        except Exception:
            manifest = {}

    existing_tests = manifest.get("tests", [])
    existing_ids = {t.get("id") for t in existing_tests}

    added_count = 0
    for t in new_tests:
        tid = t.get("id") or f"test_{len(existing_tests) + 1}"
        in_p = tests_dir / f"{tid}.in"
        out_p = tests_dir / f"{tid}.out"
        tag_p = tests_dir / f"{tid}.tag"

        in_p.write_text(t.get("input", "").strip() + "\n", encoding="utf-8")
        if t.get("expected_output"):
            out_p.write_text(t.get("expected_output", "").strip() + "\n", encoding="utf-8")
        tag_p.write_text(t.get("tag", "[PAKIET TESTÓW ZIP]"), encoding="utf-8")

        if tid not in existing_ids:
            existing_tests.append({
                "id": tid,
                "name": t.get("name", tid),
                "in_file": f"{tid}.in",
                "out_file": f"{tid}.out" if t.get("expected_output") else "",
                "tag": t.get("tag", "[PAKIET TESTÓW ZIP]")
            })
            existing_ids.add(tid)
            added_count += 1

    manifest["tests"] = existing_tests
    manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False), encoding="utf-8")

    # Zregeneruj .algo/test.sh
    time_limit = float(manifest.get("time_limit_sec", 1.0))
    title = manifest.get("title", pid.upper())
    WorkspaceBuilder.generate_test_script(pdir, pid, title, time_limit)

    # Odśwież widok StreamControllera
    try:
        streamcontroller_bridge.sync_all_problems()
    except Exception as e:
        logger.warning(f"Błąd sync_all_problems: {e}")

    return {
        "success": True,
        "problem_id": pid,
        "tests_added": added_count,
        "total_tests": len(existing_tests),
        "message": f"Pomyślnie dodano {added_count} testów do zadania {pid.upper()}!"
    }

@app.post("/api/auto-import-pdf")
async def auto_import_pdf(payload: Dict[str, Any] = Body(...)):
    """
    Automatyczny import pliku PDF pobranego przez przeglądarkę.
    Odczytuje plik z dysku, tworzy workspace lub przygotowuje metadane.
    """
    file_path_str = payload.get("file_path", "").strip()
    if not file_path_str:
        raise HTTPException(status_code=400, detail="Brak ścieżki pliku (file_path).")

    pdf_path = Path(file_path_str).expanduser().resolve()
    if not pdf_path.exists() or not pdf_path.is_file():
        raise HTTPException(status_code=404, detail=f"Plik PDF nie istnieje: {pdf_path}")

    try:
        text = PDFParser.extract_text(pdf_path)
        pdf_info = PDFParser.parse_heuristics(text)
    except Exception as e:
        logger.error(f"Błąd parsowania PDF {pdf_path}: {e}")
        raise HTTPException(status_code=400, detail=f"Nie udało się odczytać pliku PDF: {e}")

    # Zapisz w stagingu
    STAGING_DIR.mkdir(parents=True, exist_ok=True)
    staged_pdf = STAGING_DIR / "staged_pdf.pdf"
    shutil.copy2(pdf_path, staged_pdf)

    # Przygotuj metadane
    pid = payload.get("problem_id") or pdf_info.get("problem_id") or pdf_path.stem.lower()
    pid = re.sub(r'[^a-zA-Z0-9_-]', '', pid.lower()) or "zad"
    title = payload.get("title") or pdf_info.get("title") or pid.capitalize()
    time_limit = float(payload.get("time_limit") or pdf_info.get("time_limit_sec") or 1.0)
    memory_limit = int(payload.get("memory_limit") or pdf_info.get("memory_limit_mb") or 128)
    tests = pdf_info.get("tests", [])

    meta = {
        "problem_id": pid,
        "title": title,
        "time_limit_sec": time_limit,
        "memory_limit_mb": memory_limit,
        "pdf_filename": pdf_path.name,
        "pdf_size": pdf_path.stat().st_size
    }
    (STAGING_DIR / "meta.json").write_text(json.dumps(meta, indent=2), encoding="utf-8")

    auto_create = payload.get("auto_create", False)
    if auto_create:
        analysis = {
            "problem_id": pid,
            "title": title,
            "time_limit_sec": time_limit,
            "memory_limit_mb": memory_limit,
            "tests": tests
        }
        problem_dir = workspace_builder.create_problem_workspace(analysis)
        streamdeck_controller.set_active_problem(pid)
        streamcontroller_bridge.sync_all_problems()
        streamcontroller_bridge.switch_to_page(pid)

        # Wyczyść staging
        staged_pdf.unlink(missing_ok=True)
        (STAGING_DIR / "staged_zip.zip").unlink(missing_ok=True)
        (STAGING_DIR / "meta.json").unlink(missing_ok=True)

        return {
            "success": True,
            "created": True,
            "problem_id": pid,
            "title": title,
            "workspace_dir": str(problem_dir),
            "tests_count": len(tests)
        }

    return {
        "success": True,
        "created": False,
        "problem_id": pid,
        "title": title,
        "time_limit_sec": time_limit,
        "memory_limit_mb": memory_limit,
        "tests_count": len(tests),
        "pdf_filename": pdf_path.name
    }

@app.post("/api/set-active/{problem_id}")
def set_active_problem(problem_id: str):
    pid = problem_id.lower()
    streamdeck_controller.set_active_problem(pid)
    pdir = executor.get_problem_dir(pid)
    main_cpp = pdir / f"{pid}.cpp"
    workspace_builder.open_in_vscode(pdir, main_cpp)
    return {"success": True, "active": pid, "keys": streamdeck_controller.keys_state}

@app.post("/api/open-vscode/{problem_id}")
def open_vscode(problem_id: str):
    pid = problem_id.lower()
    pdir = executor.get_problem_dir(pid)
    main_cpp = pdir / f"{pid}.cpp"
    workspace_builder.open_in_vscode(pdir, main_cpp)
    return {"success": True}

@app.post("/api/switch-task/{direction}")
def switch_task(direction: str = "next"):
    curr = streamdeck_controller.active_problem_id or ""
    import subprocess
    subprocess.run(["bash", "-c", f"$HOME/.local/bin/sd_algo_switch.sh {direction} '{curr}'"], timeout=3)
    return {"success": True}

@app.post("/api/streamdeck/menu")
def open_streamdeck_menu():
    import subprocess
    subprocess.run(["bash", "-c", "$HOME/.local/bin/sd_algo_switch.sh menu"], timeout=3)
    return {"success": True}

# ----------------- Kompilacja i Uruchamianie -----------------

@app.post("/api/compile/{problem_id}")
def compile_code(problem_id: str, debug: bool = False):
    return executor.compile(problem_id, debug_mode=debug)

@app.post("/api/run-test/{problem_id}/{test_id}")
def run_single_test(problem_id: str, test_id: str, debug: bool = False):
    return executor.run_single_test(problem_id, test_id, is_debug=debug)

@app.post("/api/run-all/{problem_id}")
def run_all_tests(problem_id: str, debug: bool = False):
    return executor.run_all_tests(problem_id, is_debug=debug)

@app.post("/api/copy-input/{problem_id}/{test_id}")
def copy_input(problem_id: str, test_id: str):
    return executor.copy_test_input(problem_id, test_id)

@app.post("/api/kill/{problem_id}")
def kill_process(problem_id: str):
    stopped = executor.kill_process(problem_id)
    pdir = executor.get_problem_dir(problem_id)
    kill_sh = pdir / ".algo" / "kill.sh"
    if kill_sh.exists():
        import subprocess
        subprocess.run(["bash", str(kill_sh)], timeout=3)
    return {"success": True, "stopped": stopped}

@app.post("/api/streamdeck/press/{key_index}")
async def press_streamdeck_key(key_index: int):
    return await streamdeck_controller.execute_key(key_index)

@app.get("/api/streamdeck/state")
def get_streamdeck_state():
    return {
        "active_problem": streamdeck_controller.active_problem_id,
        "keys": streamdeck_controller.keys_state,
        "hardware_connected": hardware_driver.is_connected
    }

# ----------------- WebSocket -----------------

@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await ws_manager.connect(websocket)
    try:
        await websocket.send_json({
            "type": "INITIAL_STATE",
            "active_problem": streamdeck_controller.active_problem_id,
            "keys": streamdeck_controller.keys_state,
            "hardware_connected": hardware_driver.is_connected
        })
        while True:
            data = await websocket.receive_text()
            try:
                msg = json.loads(data)
                if msg.get("type") == "KEY_PRESS":
                    idx = msg.get("index", 0)
                    await streamdeck_controller.execute_key(idx)
            except Exception as e:
                logger.error(f"Błąd wiadomości WS: {e}")
    except WebSocketDisconnect:
        ws_manager.disconnect(websocket)

# Serwowanie plików frontendowych
frontend_dir = Path(__file__).parent.parent / "frontend"
if frontend_dir.exists():
    app.mount("/", StaticFiles(directory=str(frontend_dir), html=True), name="frontend")
