@echo off
setlocal
set SCRIPT_DIR=%~dp0
set TOOL_DIR=%SCRIPT_DIR%..\\raid-utility-tools-main\\raid-utility-tools-main\\pl-tool
set PYTHONPATH=%SCRIPT_DIR%..;%PYTHONPATH%
if exist "%TOOL_DIR%\\main.py" (
    pushd "%TOOL_DIR%"
    python main.py
    popd
) else (
    start "" "%SCRIPT_DIR%..\\raid-utility-tools-main"
)
endlocal
