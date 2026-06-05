@echo off
rem ─────────────────────────────────────────────────────────────
rem  一龙 PC 节点 一键启动器（双击运行）
rem  - 节点已在运行 → 直接打开/跳回管理页
rem  - 节点未运行   → 读取 node-agent.env 启动后再打开管理页
rem  控制台只是临时跳板，启动后自动关闭；节点进程在后台隐藏运行。
rem ─────────────────────────────────────────────────────────────
powershell -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "%~dp0start-node-agent.ps1"
