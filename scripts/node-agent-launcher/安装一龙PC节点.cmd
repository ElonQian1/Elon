@echo off
chcp 65001 >nul
setlocal
cd /d "%~dp0"
where pwsh >nul 2>nul
if %errorlevel%==0 (
  pwsh -NoProfile -ExecutionPolicy Bypass -File "%~dp0install-elon-node.ps1" -Start
) else (
  powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install-elon-node.ps1" -Start
)
if errorlevel 1 (
  echo.
  echo 安装失败，请把本窗口截图发给一龙管理员。
  pause
)
