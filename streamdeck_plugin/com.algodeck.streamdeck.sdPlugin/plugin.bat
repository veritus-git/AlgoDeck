@echo off
setlocal
set "VENV_PY=%USERPROFILE%\.algodeck\venv\Scripts\python.exe"
if exist "%VENV_PY%" (
    "%VENV_PY%" "%~dp0plugin.py" %*
) else (
    python "%~dp0plugin.py" %*
)
