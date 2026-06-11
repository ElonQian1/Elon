param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'ElonNode'),
    [switch]$KeepInstallDir
)

$ErrorActionPreference = 'Stop'

function Write-Step($text) {
    Write-Host "[一龙PC节点] $text" -ForegroundColor Cyan
}

$InstallDir = [System.IO.Path]::GetFullPath($InstallDir)
Write-Step "停止节点进程"
Stop-Process -Name 'elon-node-agent' -Force -ErrorAction SilentlyContinue

Write-Step "移除开机自启任务"
Unregister-ScheduledTask -TaskName 'ElonNodeAgentTray' -Confirm:$false -ErrorAction SilentlyContinue

$desktop = [Environment]::GetFolderPath('Desktop')
$shortcut = Join-Path $desktop '一龙PC节点.lnk'
if (Test-Path -LiteralPath $shortcut) {
    Remove-Item -LiteralPath $shortcut -Force -ErrorAction SilentlyContinue
}

if (-not $KeepInstallDir -and (Test-Path -LiteralPath $InstallDir)) {
    Write-Step "删除安装目录：$InstallDir"
    Remove-Item -LiteralPath $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ''
Write-Host '卸载完成。账号节点凭证默认保留在 Windows 用户配置目录，重新安装后可继续使用。' -ForegroundColor Green
