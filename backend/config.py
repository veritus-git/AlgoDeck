import os
from pathlib import Path
from pydantic import BaseModel

class Settings(BaseModel):
    # Base workspace directory where problem folders will be created
    workspace_dir: Path = Path(os.path.expanduser("~/algodeck-workspace"))
    
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

settings = Settings()
