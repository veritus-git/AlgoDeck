import os
from pathlib import Path
from pydantic import BaseModel

def _load_api_key() -> str:
    # 1. Environment variable
    env_key = os.getenv("GEMINI_API_KEY", "").strip()
    if env_key:
        return env_key
    # 2. Local user configuration file
    cfg_file = Path(os.path.expanduser("~/.config/algodeck/gemini_key.txt"))
    if cfg_file.exists():
        k = cfg_file.read_text(encoding="utf-8").strip()
        if k:
            return k
    # 3. Local untracked .env file
    env_path = Path(__file__).resolve().parent.parent / ".env"
    if env_path.exists():
        for line in env_path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line.startswith("GEMINI_API_KEY="):
                return line.split("=", 1)[1].strip().strip('"').strip("'")
    return ""

class Settings(BaseModel):
    # Base workspace directory where problem folders will be created
    workspace_dir: Path = Path(os.path.expanduser("~/algodeck-workspace"))
    
    # Web server configuration
    host: str = "127.0.0.1"
    port: int = 8080
    
    # Gemini AI configuration
    gemini_model: str = "gemini-3.8-flash"
    gemini_api_key: str = _load_api_key()
    
    # C++ Compiler flags
    cxx_compiler: str = "g++"
    cxx_std: str = "c++20"
    cxx_release_flags: str = "-O3 -Wall -Wextra"
    cxx_debug_flags: str = "-g -O0 -fsanitize=address,undefined -DDEBUG -Wall -Wextra"
    
    # Default problem limits if not detected
    default_time_limit_sec: float = 1.0
    default_memory_limit_mb: int = 256
    
    # Automation flags
    auto_launch_vscode: bool = True
    enable_streamdeck_hardware: bool = True
    enable_streamcontroller_bridge: bool = True

settings = Settings()
