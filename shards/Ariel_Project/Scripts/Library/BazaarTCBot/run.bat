@echo off
setlocal
set SCRIPT_DIR=%~dp0
set PYTHONPATH=%SCRIPT_DIR%..;%PYTHONPATH%
set REPO=https://github.com/NicolasMaskal/Wizard101-Bazaar-Tc-Farming
set MAIN_PY=%SCRIPT_DIR%Wizard101-Bazaar-Tc-Farming\main.py

if exist "%MAIN_PY%" (
    echo [BazaarTCBot] Found repo, attempting to launch with Python...
    python "%MAIN_PY%"
) else (
    echo [BazaarTCBot] Repository not found. Opening repo page: %REPO%
    start "" "%REPO%"
)
endlocal
