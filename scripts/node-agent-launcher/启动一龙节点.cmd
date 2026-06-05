@echo off
chcp 65001 >nul
rem One-click launcher for the elon PC node agent (double-click to run).
rem  - If the node is already running, just open the admin page.
rem  - Otherwise load node-agent.env, start the exe hidden, then open the admin page.
rem This console window is only a launcher and closes itself; the agent runs hidden.
where pwsh >nul 2>nul
if %errorlevel%==0 (
  pwsh -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "%~dp0start-node-agent.ps1"
) else (
  powershell -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "%~dp0start-node-agent.ps1"
)
