@echo off
setlocal
set SCRIPT_DIR=%~dp0
set PYTHONPATH=%SCRIPT_DIR%..;%PYTHONPATH%
set MAIN_PY=%SCRIPT_DIR%gardening-bot-main\main.py
if exist "%MAIN_PY%" (
    python "%MAIN_PY%"
) else (
    start "" "%SCRIPT_DIR%gardening-bot-main"
)
endlocal
