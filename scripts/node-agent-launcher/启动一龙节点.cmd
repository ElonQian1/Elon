@echo off
chcp 65001 >nul
rem One-click launcher for the elon PC node agent (double-click to run).
rem  - Launches the system tray icon app (tray-launcher.ps1)
rem  - Tray icon shows node status; double-click to open admin page
rem  - Single instance: if tray is already running, just opens admin page
where pwsh >nul 2>nul
if %errorlevel%==0 (
  pwsh -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "%~dp0tray-launcher.ps1"
) else (
  powershell -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "%~dp0tray-launcher.ps1"
)
