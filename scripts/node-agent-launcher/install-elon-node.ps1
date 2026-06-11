param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'ElonNode'),
    [string]$BaseUrl = 'http://43.139.149.158:8080',
    [switch]$NoDesktopShortcut,
    [switch]$NoAutoStart,
    [switch]$Start
)

$ErrorActionPreference = 'Stop'
$script:SourceScriptDir = if ($PSScriptRoot) {
    $PSScriptRoot
} else {
    Split-Path -Parent $MyInvocation.MyCommand.Path
}

function Write-Step($text) {
    Write-Host "[一龙PC节点] $text" -ForegroundColor Cyan
}

function New-Shortcut($shortcutPath, $targetPath, $workingDir) {
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = $targetPath
    $shortcut.WorkingDirectory = $workingDir
    $shortcut.IconLocation = "$env:SystemRoot\System32\shell32.dll,13"
    $shortcut.Save()
}

function Register-AutoStart($installDir) {
    $pwshCmd = Get-Command 'pwsh' -ErrorAction SilentlyContinue
    $psExe = if ($pwshCmd) { $pwshCmd.Source } else { 'powershell' }
    $trayScript = Join-Path $installDir 'tray-launcher.ps1'
    $action = New-ScheduledTaskAction -Execute $psExe `
        -Argument "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$trayScript`""
    $trigger = New-ScheduledTaskTrigger -AtLogon -User $env:USERNAME
    $settings = New-ScheduledTaskSettingsSet `
        -ExecutionTimeLimit ([TimeSpan]::Zero) `
        -RestartCount 3 `
        -RestartInterval ([TimeSpan]::FromMinutes(1))
    Register-ScheduledTask -TaskName 'ElonNodeAgentTray' `
        -Action $action `
        -Trigger $trigger `
        -Settings $settings `
        -RunLevel Limited `
        -Force | Out-Null
}

function Invoke-NoProxyDownloadFile($url, $path) {
    $wc = New-Object System.Net.WebClient
    $wc.Proxy = [System.Net.GlobalProxySelection]::GetEmptyWebProxy()
    $wc.DownloadFile($url, $path)
}

function Resolve-PackageSource {
    $here = $script:SourceScriptDir
    $localExe = Join-Path $here 'elon-node-agent.exe'
    if (Test-Path -LiteralPath $localExe) {
        return $here
    }

    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('elon-node-client-' + [Guid]::NewGuid().ToString('N'))
    $zipPath = Join-Path $tempRoot 'elon-node-agent-windows.zip'
    $extractDir = Join-Path $tempRoot 'package'
    New-Item -ItemType Directory -Force -Path $tempRoot, $extractDir | Out-Null

    $downloadUrl = $BaseUrl.TrimEnd('/') + '/api/node-agent/download/windows-client'
    Write-Step "下载 Windows 客户端包：$downloadUrl"
    Invoke-NoProxyDownloadFile $downloadUrl $zipPath
    Expand-Archive -LiteralPath $zipPath -DestinationPath $extractDir -Force

    $extractedExe = Get-ChildItem -LiteralPath $extractDir -Recurse -Filter 'elon-node-agent.exe' |
        Select-Object -First 1
    if (-not $extractedExe) {
        throw '下载的客户端包里没有 elon-node-agent.exe'
    }
    return $extractedExe.DirectoryName
}

function Copy-ClientFiles($sourceDir, $installDir) {
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
    $files = @(
        'elon-node-agent.exe',
        'node-agent-version.json',
        'node-agent.env.example',
        'start-node-agent.ps1',
        'tray-launcher.ps1',
        'install-elon-node.ps1',
        'uninstall-elon-node.ps1',
        '启动一龙节点.cmd',
        '安装一龙PC节点.cmd',
        '卸载一龙PC节点.cmd',
        'README.txt'
    )
    foreach ($file in $files) {
        $src = Join-Path $sourceDir $file
        if (Test-Path -LiteralPath $src) {
            $dest = Join-Path $installDir $file
            if ([System.IO.Path]::GetFullPath($src) -ine [System.IO.Path]::GetFullPath($dest)) {
                Copy-Item -LiteralPath $src -Destination $dest -Force
            }
        }
    }
}

$sourceDir = Resolve-PackageSource
$InstallDir = [System.IO.Path]::GetFullPath($InstallDir)
Write-Step "安装目录：$InstallDir"

Stop-Process -Name 'elon-node-agent' -Force -ErrorAction SilentlyContinue
Copy-ClientFiles $sourceDir $InstallDir

if (-not (Test-Path -LiteralPath (Join-Path $InstallDir 'elon-node-agent.exe'))) {
    throw '安装失败：缺少 elon-node-agent.exe'
}

if (-not $NoDesktopShortcut) {
    $desktop = [Environment]::GetFolderPath('Desktop')
    New-Shortcut `
        -shortcutPath (Join-Path $desktop '一龙PC节点.lnk') `
        -targetPath (Join-Path $InstallDir '启动一龙节点.cmd') `
        -workingDir $InstallDir
    Write-Step '已创建桌面快捷方式'
}

if (-not $NoAutoStart) {
    Register-AutoStart $InstallDir
    Write-Step '已启用开机自启'
}

if ($Start) {
    $launcher = Join-Path $InstallDir '启动一龙节点.cmd'
    Start-Process -FilePath $launcher -WorkingDirectory $InstallDir
    Write-Step '已启动节点托盘，浏览器会打开本地管理页'
}

Write-Host ''
Write-Host '安装完成。首次使用请在打开的本地管理页登录一龙账号，登录后本机会自动注册为 PC 节点。' -ForegroundColor Green
