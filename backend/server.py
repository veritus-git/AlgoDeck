import asyncio
import json
import logging
import os
import shutil
from pathlib import Path
from typing import Dict, Any, List, Optional

from fastapi import FastAPI, UploadFile, File, Form, WebSocket, WebSocketDisconnect, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import HTMLResponse, JSONResponse, FileResponse
from fastapi.staticfiles import StaticFiles

from backend.agent.gemini_agent import GeminiAgent
from backend.config import settings
from backend.runner.executor import TestExecutor
from backend.streamdeck.controller import StreamDeckController
from backend.streamdeck.hardware import StreamDeckHardwareDriver
from backend.streamdeck.streamcontroller_bridge import StreamControllerBridge
from backend.workspace.builder import WorkspaceBuilder

# Logging setup
logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(name)s: %(message)s")
logger = logging.getLogger("algodeck.server")

app = FastAPI(title="AlgoDeck - Competitive Programming Environment", version="1.0.0")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Core subsystems
executor = TestExecutor()
workspace_builder = WorkspaceBuilder()
streamdeck_controller = StreamDeckController(executor)
streamcontroller_bridge = StreamControllerBridge()
hardware_driver = StreamDeckHardwareDriver(streamdeck_controller)
agent = GeminiAgent()

# WebSockets connection manager
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

# Hook controller state updates into WebSocket broadcast
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
    logger.info("Initializing AlgoDeck background services...")
    loop = asyncio.get_running_loop()
    hardware_driver.start(loop)

@app.on_event("shutdown")
def shutdown_event():
    hardware_driver.close()

# ----------------- REST Endpoints -----------------

@app.post("/api/upload-pdf")
async def upload_pdf(file: UploadFile = File(...), api_key: Optional[str] = Form(None)):
    """Receives task PDF, runs Gemini agent, scaffolds workspace, creates Stream Deck profile, and launches VS Code."""
    logger.info(f"Received PDF upload: {file.filename}")
    
    # Save file temporarily
    temp_dir = Path("/tmp/algodeck_uploads")
    temp_dir.mkdir(parents=True, exist_ok=True)
    temp_pdf = temp_dir / file.filename
    with open(temp_pdf, "wb") as buffer:
        shutil.copyfileobj(file.file, buffer)

    await ws_manager.broadcast({"type": "PIPELINE_STEP", "step": "PDF_SAVED", "message": "Plik PDF zapisany pomyślnie."})

    # Step 1: Gemini AI Analysis
    await ws_manager.broadcast({"type": "PIPELINE_STEP", "step": "AI_ANALYSIS", "message": "Analiza treści zadania przez Agenta Gemini..."})
    if api_key:
        active_agent = GeminiAgent(api_key=api_key)
    else:
        active_agent = agent

    analysis = active_agent.analyze_pdf(temp_pdf)
    problem_id = analysis.get("problem_id", "zad").lower()

    # Step 2: Build Workspace & Code Templates
    await ws_manager.broadcast({"type": "PIPELINE_STEP", "step": "WORKSPACE_GENERATION", "message": f"Tworzenie katalogu roboczego i szablonu C++ dla '{problem_id}'..."})
    problem_dir = workspace_builder.create_problem_workspace(analysis, original_pdf=temp_pdf)

    # Step 3: Configure Stream Deck
    await ws_manager.broadcast({"type": "PIPELINE_STEP", "step": "STREAMDECK_SETUP", "message": "Konfiguracja 15 przycisków Stream Decka..."})
    streamdeck_controller.set_active_problem(problem_id)
    streamcontroller_bridge.generate_page_for_problem(problem_id, analysis)

    # Step 4: Ready
    await ws_manager.broadcast({
        "type": "PIPELINE_STEP",
        "step": "COMPLETE",
        "message": "Środowisko gotowe! VS Code uruchomiony.",
        "problem_id": problem_id
    })

    return {
        "success": True,
        "problem_id": problem_id,
        "analysis": analysis,
        "workspace_path": str(problem_dir.resolve()),
        "keys": streamdeck_controller.keys_state
    }

@app.get("/api/problems")
def list_problems():
    """Lists all problem workspaces."""
    wdir = settings.workspace_dir
    problems = []
    if wdir.exists():
        for p in wdir.iterdir():
            if not p.is_dir():
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
    return {"problems": problems, "active": streamdeck_controller.active_problem_id}

@app.get("/api/problem/{problem_id}")
def get_problem(problem_id: str):
    manifest = executor.get_manifest(problem_id)
    if not manifest:
        raise HTTPException(status_code=404, detail="Problem not found")
    
    pdir = executor.get_problem_dir(problem_id)
    src_file = pdir / f"{problem_id}.cpp"
    source_code = src_file.read_text(encoding="utf-8") if src_file.exists() else ""

    return {
        "manifest": manifest,
        "source_code": source_code,
        "is_active": streamdeck_controller.active_problem_id == problem_id.lower()
    }

@app.post("/api/set-active/{problem_id}")
def set_active_problem(problem_id: str):
    pid = problem_id.lower()
    streamdeck_controller.set_active_problem(pid)
    manifest = executor.get_manifest(pid)
    streamcontroller_bridge.generate_page_for_problem(pid, manifest)
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

@app.post("/api/compile/{problem_id}")
def compile_code(problem_id: str, debug: bool = False):
    res = executor.compile(problem_id, debug_mode=debug)
    return res

@app.post("/api/run-test/{problem_id}/{test_id}")
def run_single_test(problem_id: str, test_id: str, debug: bool = False):
    res = executor.run_single_test(problem_id, test_id, is_debug=debug)
    return res

@app.post("/api/run-all/{problem_id}")
def run_all_tests(problem_id: str, debug: bool = False):
    res = executor.run_all_tests(problem_id, is_debug=debug)
    return res

@app.post("/api/copy-input/{problem_id}/{test_id}")
def copy_input(problem_id: str, test_id: str):
    res = executor.copy_test_input(problem_id, test_id)
    return res

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
    """Triggers execution of action associated with key index (0-14)."""
    res = await streamdeck_controller.execute_key(key_index)
    return res

@app.get("/api/streamdeck/state")
def get_streamdeck_state():
    return {
        "active_problem": streamdeck_controller.active_problem_id,
        "keys": streamdeck_controller.keys_state,
        "hardware_connected": hardware_driver.is_connected
    }

@app.get("/api/settings")
def get_settings():
    return {
        "workspace_dir": str(settings.workspace_dir),
        "cxx_compiler": settings.cxx_compiler,
        "cxx_release_flags": settings.cxx_release_flags,
        "cxx_debug_flags": settings.cxx_debug_flags,
        "gemini_model": settings.gemini_model,
        "has_api_key": bool(settings.gemini_api_key),
        "auto_launch_vscode": settings.auto_launch_vscode,
        "hardware_connected": hardware_driver.is_connected
    }

@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await ws_manager.connect(websocket)
    try:
        # Send current state upon connection
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
                logger.error(f"Error handling WS message: {e}")
    except WebSocketDisconnect:
        ws_manager.disconnect(websocket)

# Serve Frontend static files
frontend_dir = Path(__file__).parent.parent / "frontend"
if frontend_dir.exists():
    app.mount("/", StaticFiles(directory=str(frontend_dir), html=True), name="frontend")
