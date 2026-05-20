@echo off
setlocal
set SCRIPT_DIR=%~dp0
set PYTHONPATH=%SCRIPT_DIR%..;%PYTHONPATH%
set MAIN_PY=%SCRIPT_DIR%wizSDK-master\main.py
set README=%SCRIPT_DIR%wizSDK-master\README.md

if exist "%MAIN_PY%" (
    echo [wizSDK] Launching main.py...
    python "%MAIN_PY%"
) else (
    if exist "%README%" (
        start "" "%README%"
    ) else (
        start "" "%SCRIPT_DIR%wizSDK-master"
    )
)
endlocal
