@echo off
setlocal
set LOGFILE=%~dp0sample_echo.log
echo [%date% %time%] Sample Echo is running...> "%LOGFILE%"
timeout /t 2 >nul
echo [%date% %time%] Sample Echo done.>> "%LOGFILE%"
endlocal
