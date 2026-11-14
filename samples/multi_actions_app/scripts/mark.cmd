@echo off
setlocal
set "OUT=%~1"
set "TAG=%~2"
if not exist "%~dp1" mkdir "%~dp1" >nul 2>nul
>"%OUT%" echo %TAG%
echo marker:%TAG%
