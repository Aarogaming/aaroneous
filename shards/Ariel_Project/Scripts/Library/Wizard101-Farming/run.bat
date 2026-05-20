@echo off
setlocal
set SCRIPT_DIR=%~dp0
set PYTHONPATH=%SCRIPT_DIR%..;%PYTHONPATH%
set README=%SCRIPT_DIR%Wizard101-Farming-main\README.md
set MAIN_PY=%SCRIPT_DIR%Wizard101-Farming-main\main.py

if exist "%MAIN_PY%" (
    echo [Wizard101-Farming] Launching main.py...
    python "%MAIN_PY%"
) else (
    if exist "%README%" (
        start "" "%README%"
    ) else (
        start "" "%SCRIPT_DIR%Wizard101-Farming-main"
    )
)
endlocal
