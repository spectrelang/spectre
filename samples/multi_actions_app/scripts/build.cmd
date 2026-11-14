@echo off
setlocal
if "%SPECTRE_BIN%"=="" set "SPECTRE_BIN=spectre"
"%SPECTRE_BIN%" app.sx %*
