import json
import os
from pathlib import Path
from pydantic import BaseModel

CONFIG_FILE = Path(os.path.expanduser("~/.config/algodeck/settings.json"))

def _load_settings_dict():
    defaults = {
        "workspace_dir": os.path.expanduser("~/algodeck-workspace"),
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

_cfg = _load_settings_dict()

class Settings(BaseModel):
    # Base workspace directory where problem folders will be created
    workspace_dir: Path = Path(_cfg.get("workspace_dir", os.path.expanduser("~/algodeck-workspace")))
    vscode_mode: str = _cfg.get("vscode_mode", "single_window")
    notifications: bool = _cfg.get("notifications", True)
    
    # Web server configuration
    host: str = "127.0.0.1"
    port: int = 8080
    
    # C++ Compiler flags
    cxx_compiler: str = "g++"
    cxx_std: str = "c++20"
    cxx_release_flags: str = "-O3 -Wall -Wextra"
    cxx_debug_flags: str = "-g -O0 -fsanitize=address,undefined -DDEBUG -Wall -Wextra"
    
    # Default problem limits if not detected
    default_time_limit_sec: float = 1.0
    default_memory_limit_mb: int = 128
    
    # Automation flags
    auto_launch_vscode: bool = True
    enable_streamdeck_hardware: bool = True
    enable_streamcontroller_bridge: bool = True

    def reload(self):
        new_cfg = _load_settings_dict()
        self.workspace_dir = Path(new_cfg.get("workspace_dir", os.path.expanduser("~/algodeck-workspace")))
        self.vscode_mode = new_cfg.get("vscode_mode", "single_window")
        self.notifications = new_cfg.get("notifications", True)

settings = Settings()
