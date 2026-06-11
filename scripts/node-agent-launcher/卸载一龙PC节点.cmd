@echo off
chcp 65001 >nul
setlocal
set SCRIPT_DIR=%~dp0
cd /d "%TEMP%"
where pwsh >nul 2>nul
if %errorlevel%==0 (
  pwsh -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%uninstall-elon-node.ps1"
) else (
  powershell -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%uninstall-elon-node.ps1"
)
if errorlevel 1 (
  echo.
  echo 卸载失败，请把本窗口截图发给一龙管理员。
  pause
)
