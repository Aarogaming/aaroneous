@echo off
setlocal
set SCRIPT_DIR=%~dp0
set PYTHONPATH=%SCRIPT_DIR%..;%PYTHONPATH%
set REPO=https://github.com/hikarihacks/FreekiGames-Trivia-Bot
set MAIN_PY=%SCRIPT_DIR%FreekiGames-Trivia-Bot\main.py

if exist "%MAIN_PY%" (
    echo [CrownsTriviaBot] Found repo, attempting to launch with Python...
    python "%MAIN_PY%"
) else (
    echo [CrownsTriviaBot] Repository not found. Opening repo page: %REPO%
    start "" "%REPO%"
)
endlocal
