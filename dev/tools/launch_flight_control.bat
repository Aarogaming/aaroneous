@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
cd /d "%SCRIPT_DIR%..\.."
start "" "%SCRIPT_DIR%afc\target\release\afc.exe" --gui
