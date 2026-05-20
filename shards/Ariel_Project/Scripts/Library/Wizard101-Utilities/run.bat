@echo off
setlocal
set SCRIPT_DIR=%~dp0
set README=%SCRIPT_DIR%Wizard101-Utilities-master\README.md

if exist "%README%" (
    start "" "%README%"
) else (
    start "" "%SCRIPT_DIR%Wizard101-Utilities-master"
)
endlocal
