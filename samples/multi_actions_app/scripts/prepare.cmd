@echo off
setlocal
set "MODE=%~1"
set "OUT=%~2"
if not "%OUT%"=="" (
    if not exist "%~dp2" mkdir "%~dp2" >nul 2>nul
    >"%OUT%" echo prepare:%MODE%
)
echo prepare:%MODE%
