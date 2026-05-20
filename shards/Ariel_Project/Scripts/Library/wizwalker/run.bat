@echo off
setlocal
set SCRIPT_DIR=%~dp0
set PYTHONPATH=%SCRIPT_DIR%..;%PYTHONPATH%
set MAIN_PY=%SCRIPT_DIR%wizwalker-master\main.py
set README=%SCRIPT_DIR%wizwalker-master\README.md

if exist "%MAIN_PY%" (
    echo [wizwalker] Launching main.py...
    python "%MAIN_PY%"
) else (
    if exist "%README%" (
        start "" "%README%"
    ) else (
        start "" "%SCRIPT_DIR%wizwalker-master"
    )
)
endlocal
