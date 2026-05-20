@echo off
setlocal
set SCRIPT_DIR=%~dp0
set PYTHONPATH=%SCRIPT_DIR%..;%PYTHONPATH%
set BRIDGE=%SCRIPT_DIR%automatus_bridge.py

if exist "%BRIDGE%" (
    python "%BRIDGE%"
) else (
    start "" "%SCRIPT_DIR%"
)
endlocal
