@echo off
setlocal
chcp 65001 >nul
title PERCI // dark-blood
set "ROOT=%~dp0"
set "PYTHONUTF8=1"
rem -NoLogo suppresses extra PS noise; Launch-Perci clears the rest before chat.
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%ROOT%Launch-Perci.ps1" %*
endlocal