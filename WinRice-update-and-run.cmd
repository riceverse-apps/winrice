@echo off
REM Double-click: updates WinRice from GitHub, rebuilds if needed and opens it.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\update-and-run.ps1" %*
if errorlevel 1 pause
