@echo off
setlocal
set SCRIPT_DIR=%~dp0
set PYTHONPATH=%SCRIPT_DIR%..;%PYTHONPATH%
set README=%SCRIPT_DIR%README.md

if exist "%README%" (
    start "" "%README%"
) else (
    start "" "%SCRIPT_DIR%"
)
endlocal
