@echo off
setlocal
set SCRIPT_DIR=%~dp0
set PYTHONPATH=%SCRIPT_DIR%..;%PYTHONPATH%
set REPO=https://github.com/ThatmewH/Wizard101Bot-Selenium-Captcha
set MAIN_PY=%SCRIPT_DIR%Wizard101Bot-Selenium-Captcha\main.py

if exist "%MAIN_PY%" (
    echo [SeleniumCaptchaBot] Found repo, attempting to launch with Python...
    python "%MAIN_PY%"
) else (
    echo [SeleniumCaptchaBot] Repository not found. Opening repo page: %REPO%
    start "" "%REPO%"
)
endlocal
