# tray-launcher.ps1 — 一龙 PC 节点托盘启动器
#
# 功能：
#   - 系统托盘图标实时显示节点状态（绿=已连接 橙=运行中 红=已停止）
#   - 双击图标 → 打开 http://127.0.0.1:7799/ 管理页
#   - 右键菜单：打开管理页 / 重启节点 / 开机自启（开关）/ 退出
#   - 启动时自动拉起节点进程（如未运行）
#   - 单实例保护：已有托盘进程时直接打开管理页并退出
#
# 用法：双击「启动一龙节点.cmd」即可（不要直接双击本 .ps1）

$here = Split-Path -Parent $MyInvocation.MyCommand.Path

function Load-EnvFile($path) {
    if (-not (Test-Path -LiteralPath $path)) { return }
    foreach ($raw in Get-Content -LiteralPath $path) {
        $line = $raw.Trim()
        if (-not $line -or $line.StartsWith('#') -or -not $line.Contains('=')) { continue }
        $idx = $line.IndexOf('=')
        $key = $line.Substring(0, $idx).Trim()
        $val = $line.Substring($idx + 1).Trim()
        if (($val.StartsWith('"') -and $val.EndsWith('"')) -or ($val.StartsWith("'") -and $val.EndsWith("'"))) {
            if ($val.Length -ge 2) { $val = $val.Substring(1, $val.Length - 2) }
        }
        if ($key) { [Environment]::SetEnvironmentVariable($key, $val, 'Process') }
    }
}

Load-EnvFile (Join-Path $here 'node-agent.env')
$port = if ($env:NODE_ADMIN_PORT) { $env:NODE_ADMIN_PORT } else { "7799" }
$adminUrl = "http://127.0.0.1:$port/"

# ── 单实例保护（命名 Mutex）──────────────────────────────────────────────────
Add-Type -TypeDefinition @"
using System.Threading;
public static class ElonTrayMutex {
    public static readonly Mutex M = new Mutex(false, "Global\\ElonNodeAgentTray");
    public static bool TryAcquire() { return M.WaitOne(0, false); }
}
"@ -Language CSharp -ErrorAction SilentlyContinue

if (-not [ElonTrayMutex]::TryAcquire()) {
    Start-Process $adminUrl
    exit 0
}

# ── 基础组件 ──────────────────────────────────────────────────────────────────
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$taskName = "ElonNodeAgentTray"

# ── 图标创建 ──────────────────────────────────────────────────────────────────
function New-StatusIcon([int]$r, [int]$g, [int]$b) {
    $bmp = New-Object System.Drawing.Bitmap(16, 16)
    $gr  = [System.Drawing.Graphics]::FromImage($bmp)
    $gr.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    # 白色外环
    $gr.FillEllipse(
        (New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(220, 220, 220))),
        0, 0, 15, 15)
    # 彩色内圆
    $gr.FillEllipse(
        (New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb($r, $g, $b))),
        2, 2, 11, 11)
    $gr.Dispose()
    $icon = [System.Drawing.Icon]::FromHandle($bmp.GetHicon())
    $bmp.Dispose()
    return $icon
}

$script:icGreen  = New-StatusIcon 46  160 67     # 已连接（绿）
$script:icOrange = New-StatusIcon 210 153 34     # 运行中未连（橙）
$script:icRed    = New-StatusIcon 200  55 55     # 已停止（红）

# ── 节点管理 ──────────────────────────────────────────────────────────────────
function Get-ExePath {
    $p = $env:NODE_AGENT_EXE
    if ($p -and (Test-Path $p)) { return $p }
    return Join-Path $here "elon-node-agent.exe"
}

function Get-UpdateBaseUrl {
    if ($env:NODE_AGENT_UPDATE_BASE_URL) { return $env:NODE_AGENT_UPDATE_BASE_URL.TrimEnd('/') }
    if ($env:NODE_CLOUD_HTTP_URL) { return $env:NODE_CLOUD_HTTP_URL.TrimEnd('/') }
    if ($env:NODE_CLOUD_URL) {
        $cloud = $env:NODE_CLOUD_URL
        if ($cloud.StartsWith('wss://')) {
            return ('https://' + $cloud.Substring(6).Split('/')[0]).TrimEnd('/')
        }
        if ($cloud.StartsWith('ws://')) {
            return ('http://' + $cloud.Substring(5).Split('/')[0]).TrimEnd('/')
        }
    }
    return 'http://43.139.149.158:8080'
}

function Invoke-NoProxyDownloadString($url) {
    $wc = New-Object System.Net.WebClient
    $wc.Proxy = [System.Net.GlobalProxySelection]::GetEmptyWebProxy()
    $wc.Headers.Add('Accept', 'application/json')
    return $wc.DownloadString($url)
}

function Invoke-NoProxyDownloadFile($url, $path) {
    $wc = New-Object System.Net.WebClient
    $wc.Proxy = [System.Net.GlobalProxySelection]::GetEmptyWebProxy()
    $wc.DownloadFile($url, $path)
}

function Get-ManagedClientFiles {
    return @(
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
}

function Copy-ManagedClientFiles($sourceDir) {
    foreach ($file in Get-ManagedClientFiles) {
        $src = Join-Path $sourceDir $file
        if (-not (Test-Path -LiteralPath $src)) { continue }
        $dest = if ($file -eq 'elon-node-agent.exe') { Get-ExePath } else { Join-Path $here $file }
        $srcFull = [System.IO.Path]::GetFullPath($src)
        $destFull = [System.IO.Path]::GetFullPath($dest)
        if ($srcFull -ieq $destFull) { continue }
        Copy-Item -LiteralPath $src -Destination $dest -Force
    }
}

function Update-AgentExeFromVersion($remote, $remoteText, $baseUrl, $versionFile) {
    $exe = Get-ExePath
    $downloadUrl = if ($remote.downloadUrl) { $remote.downloadUrl } else { "$baseUrl/api/node-agent/download/windows" }
    $tmp = "$exe.new"
    Invoke-NoProxyDownloadFile $downloadUrl $tmp
    if (-not (Test-Path -LiteralPath $tmp)) { return $false }
    $size = (Get-Item -LiteralPath $tmp).Length
    if ($size -lt 1024 * 1024) {
        Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
        return $false
    }
    Move-Item -LiteralPath $tmp -Destination $exe -Force
    $remoteText | Set-Content -LiteralPath $versionFile -Encoding UTF8
    return $true
}

function Update-ClientPackageFromVersion($remote, $remoteText, $baseUrl, $versionFile) {
    $downloadUrl = if ($remote.windowsClientDownloadUrl) {
        $remote.windowsClientDownloadUrl
    } else {
        "$baseUrl/api/node-agent/download/windows-client"
    }
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('elon-node-client-update-' + [Guid]::NewGuid().ToString('N'))
    $zipPath = Join-Path $tempRoot 'elon-node-agent-windows.zip'
    $extractDir = Join-Path $tempRoot 'package'
    New-Item -ItemType Directory -Force -Path $tempRoot, $extractDir | Out-Null
    try {
        Invoke-NoProxyDownloadFile $downloadUrl $zipPath
        if ((Get-Item -LiteralPath $zipPath).Length -lt 1024 * 1024) { return $false }
        Expand-Archive -LiteralPath $zipPath -DestinationPath $extractDir -Force
        $packageDir = Get-ChildItem -LiteralPath $extractDir -Recurse -Filter 'elon-node-agent.exe' |
            Select-Object -First 1 |
            ForEach-Object { $_.DirectoryName }
        if (-not $packageDir) { return $false }
        $packageExe = Join-Path $packageDir 'elon-node-agent.exe'
        if ((Get-Item -LiteralPath $packageExe).Length -lt 1024 * 1024) { return $false }
        Copy-ManagedClientFiles $packageDir
        $remoteText | Set-Content -LiteralPath $versionFile -Encoding UTF8
        return $true
    } finally {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Update-ClientIfNeeded {
    if ($env:NODE_AGENT_AUTO_UPDATE -match '^(0|false|no|off)$') { return }
    $exe = Get-ExePath
    $baseUrl = Get-UpdateBaseUrl
    $versionUrl = "$baseUrl/api/node-agent/version"
    $versionFile = Join-Path $here 'node-agent-version.json'
    try {
        $remoteText = Invoke-NoProxyDownloadString $versionUrl
        $remote = $remoteText | ConvertFrom-Json
        $local = $null
        if (Test-Path -LiteralPath $versionFile) {
            $local = Get-Content -Raw -LiteralPath $versionFile | ConvertFrom-Json
        }
        $sameGitSha = $local -and $remote.gitSha -and $local.gitSha -eq $remote.gitSha
        if ((Test-Path -LiteralPath $exe) -and $sameGitSha) { return }

        $updated = Update-ClientPackageFromVersion $remote $remoteText $baseUrl $versionFile
        if (-not $updated) {
            Update-AgentExeFromVersion $remote $remoteText $baseUrl $versionFile | Out-Null
        }
    } catch {
        Remove-Item -LiteralPath "$exe.new" -Force -ErrorAction SilentlyContinue
    }
}

function Start-NodeIfNeeded {
    $proc = Get-Process -Name "elon-node-agent" -ErrorAction SilentlyContinue
    if ($proc) { return }
    Update-ClientIfNeeded
    $exe = Get-ExePath
    if (Test-Path $exe) {
        [Environment]::SetEnvironmentVariable("NODE_AUTO_OPEN_ADMIN", "0", 'Process')
        Start-Process $exe -WorkingDirectory $here -WindowStyle Hidden
    }
}

function Restart-Node {
    Stop-Process -Name "elon-node-agent" -Force -ErrorAction SilentlyContinue
    Stop-TtsWorkerIfRunning
    Start-Sleep -Milliseconds 1000
    Start-NodeIfNeeded
    Start-TtsWorkerIfNeeded
}

# ── TTS Worker（可选伴生进程）────────────────────────────────────────────
$script:TtsProc = $null

function Resolve-TtsPythonExe {
    if ($env:TTS_PYTHON_EXE -and (Test-Path $env:TTS_PYTHON_EXE)) { return $env:TTS_PYTHON_EXE }
    $repoRoot = try { (git -C $here rev-parse --show-toplevel 2>$null) } catch { $null }
    if ($repoRoot) {
        $venvPy = Join-Path $repoRoot.Trim() ".runtime\tts-worker-model\venv\Scripts\python.exe"
        if (Test-Path $venvPy) { return $venvPy }
    }
    return $null
}

function Start-TtsWorkerIfNeeded {
    if ($env:TTS_WORKER_ENABLED -notmatch '^(1|true|yes|on)$') { return }
    if ($script:TtsProc -and -not $script:TtsProc.HasExited) { return }

    $pyExe = Resolve-TtsPythonExe
    if (-not $pyExe) { return }

    $repoRoot = try { (git -C $here rev-parse --show-toplevel 2>$null).Trim() } catch { $null }
    if (-not $repoRoot) { return }
    $workerDir = Join-Path $repoRoot "server\tts_worker"
    if (-not (Test-Path (Join-Path $workerDir "model_tts_worker.py"))) { return }

    $port     = if ($env:TTS_WORKER_PORT) { $env:TTS_WORKER_PORT } else { "5011" }
    $provider = if ($env:TTS_PROVIDER)    { $env:TTS_PROVIDER }    else { "index_tts2" }
    $assets   = if ($env:TTS_ASSET_ROOT)  { $env:TTS_ASSET_ROOT }  else { Join-Path $repoRoot "server\assets\tts" }

    [Environment]::SetEnvironmentVariable("ELON_TTS_WORKER_HOST",    "127.0.0.1",  'Process')
    [Environment]::SetEnvironmentVariable("ELON_TTS_WORKER_PORT",    $port,        'Process')
    [Environment]::SetEnvironmentVariable("ELON_TTS_PROVIDER",       $provider,    'Process')
    [Environment]::SetEnvironmentVariable("ELON_TTS_MODEL_PROVIDER", $provider,    'Process')
    [Environment]::SetEnvironmentVariable("ELON_TTS_ASSET_ROOT",     $assets,      'Process')
    if ($env:TTS_INDEXTTS2_MODEL_DIR) { [Environment]::SetEnvironmentVariable("ELON_INDEXTTS2_MODEL_DIR", $env:TTS_INDEXTTS2_MODEL_DIR, 'Process') }
    if ($env:TTS_INDEXTTS2_CFG_PATH)  { [Environment]::SetEnvironmentVariable("ELON_INDEXTTS2_CFG_PATH",  $env:TTS_INDEXTTS2_CFG_PATH,  'Process') }
    if ($env:TTS_COSYVOICE_REPO_DIR)  { [Environment]::SetEnvironmentVariable("ELON_COSYVOICE_REPO_DIR",  $env:TTS_COSYVOICE_REPO_DIR,  'Process') }
    if ($env:TTS_COSYVOICE_MODEL_DIR) { [Environment]::SetEnvironmentVariable("ELON_COSYVOICE_MODEL_DIR", $env:TTS_COSYVOICE_MODEL_DIR, 'Process') }

    $script:TtsProc = Start-Process -FilePath $pyExe `
        -ArgumentList "-m", "uvicorn", "model_tts_worker:app", "--host", "127.0.0.1", "--port", $port `
        -WorkingDirectory $workerDir -WindowStyle Hidden -PassThru -ErrorAction SilentlyContinue
}

function Stop-TtsWorkerIfRunning {
    if ($script:TtsProc -and -not $script:TtsProc.HasExited) {
        try { $script:TtsProc.Kill() } catch {}
    }
    $script:TtsProc = $null
}

function Get-NodeStatus {
    try {
        $wc = New-Object System.Net.WebClient
        $wc.Proxy = [System.Net.GlobalProxySelection]::GetEmptyWebProxy()
        $wc.Headers.Add("Accept", "application/json")
        return $wc.DownloadString("http://127.0.0.1:$port/api/status") | ConvertFrom-Json
    } catch { return $null }
}

# ── 开机自启 ──────────────────────────────────────────────────────────────────
function Test-AutoStartEnabled {
    $t = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
    return ($t -ne $null -and $t.State -ne "Disabled")
}

function Toggle-AutoStart {
    if (Test-AutoStartEnabled) {
        Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
    } else {
        $pwshCmd = Get-Command "pwsh" -ErrorAction SilentlyContinue
        $pwshBin = if ($pwshCmd) { $pwshCmd.Source } else { "powershell" }
        $thisScript = Join-Path $here "tray-launcher.ps1"
        $act = New-ScheduledTaskAction -Execute $pwshBin `
            -Argument "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$thisScript`""
        $tri = New-ScheduledTaskTrigger -AtLogon -User $env:USERNAME
        $set = New-ScheduledTaskSettingsSet `
            -ExecutionTimeLimit ([TimeSpan]::Zero) `
            -RestartCount 3 `
            -RestartInterval ([TimeSpan]::FromMinutes(1))
        Register-ScheduledTask -TaskName $taskName `
            -Action $act -Trigger $tri -Settings $set `
            -RunLevel Limited -Force -ErrorAction SilentlyContinue | Out-Null
    }
}

# ── 托盘图标 ──────────────────────────────────────────────────────────────────
$tray = New-Object System.Windows.Forms.NotifyIcon
$tray.Icon = $script:icOrange
$tray.Text = "一龙节点 — 启动中…"
$tray.Visible = $true

# ── 右键菜单 ──────────────────────────────────────────────────────────────────
$cm = New-Object System.Windows.Forms.ContextMenuStrip

$miStatus = New-Object System.Windows.Forms.ToolStripMenuItem("正在检查…")
$miStatus.Enabled = $false
$cm.Items.Add($miStatus) | Out-Null
$cm.Items.Add((New-Object System.Windows.Forms.ToolStripSeparator)) | Out-Null

$miOpen = New-Object System.Windows.Forms.ToolStripMenuItem("🌐  打开管理页")
$miOpen.add_Click({ Start-Process $adminUrl })
$cm.Items.Add($miOpen) | Out-Null

$miRestart = New-Object System.Windows.Forms.ToolStripMenuItem("🔄  重启节点")
$miRestart.add_Click({ Restart-Node; Start-Sleep -Milliseconds 2000; Update-TrayStatus })
$cm.Items.Add($miRestart) | Out-Null

$cm.Items.Add((New-Object System.Windows.Forms.ToolStripSeparator)) | Out-Null

$miAutoStart = New-Object System.Windows.Forms.ToolStripMenuItem("⚡  开机自启")
$miAutoStart.Checked = Test-AutoStartEnabled
$miAutoStart.add_Click({ Toggle-AutoStart; $miAutoStart.Checked = Test-AutoStartEnabled })
$cm.Items.Add($miAutoStart) | Out-Null

$cm.Items.Add((New-Object System.Windows.Forms.ToolStripSeparator)) | Out-Null

$miExit = New-Object System.Windows.Forms.ToolStripMenuItem("✖  退出（同时停止节点）")
$miExit.add_Click({
    Stop-Process -Name "elon-node-agent" -Force -ErrorAction SilentlyContinue
    Stop-TtsWorkerIfRunning
    $tray.Visible = $false
    [System.Windows.Forms.Application]::Exit()
})
$cm.Items.Add($miExit) | Out-Null

$tray.ContextMenuStrip = $cm

# 双击打开管理页
$tray.add_DoubleClick({ Start-Process $adminUrl })

# ── 状态刷新 ──────────────────────────────────────────────────────────────────
function Update-TrayStatus {
    $proc = Get-Process -Name "elon-node-agent" -ErrorAction SilentlyContinue
    if (-not $proc) {
        $tray.Icon = $script:icRed
        $tray.Text = "一龙节点 ✗ 已停止"
        $miStatus.Text = "❌ 节点未运行，右键→重启"
        return
    }
    $st = Get-NodeStatus
    if ($st -and $st.connected -and $st.logged_in) {
        $tray.Icon = $script:icGreen
        $tray.Text = "一龙节点 ✓ 已连接"
        $agentShort = if ($st.agent_id) { " · $($st.agent_id)" } else { "" }
        $miStatus.Text = "✅ 已连接$agentShort"
    } elseif ($st -and $st.logged_in) {
        $tray.Icon = $script:icOrange
        $tray.Text = "一龙节点 ○ 连接中…"
        $miStatus.Text = "⏳ $($st.last_event)"
    } elseif ($st) {
        $tray.Icon = $script:icOrange
        $tray.Text = "一龙节点 ○ 未登录"
        $miStatus.Text = "⚠️  未登录，双击打开管理页登录"
    } else {
        $tray.Icon = $script:icOrange
        $tray.Text = "一龙节点 ○ 管理页启动中…"
        $miStatus.Text = "⏳ 等待节点就绪…"
    }
}

# 定时刷新（每 8 秒）
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 8000
$timer.add_Tick({ Update-TrayStatus })
$timer.Start()

# ── 启动 ──────────────────────────────────────────────────────────────────────
Start-NodeIfNeeded
Start-TtsWorkerIfNeeded
Start-Process $adminUrl          # 首次启动时自动打开管理页
Start-Sleep -Milliseconds 2500
Update-TrayStatus

# ── 消息循环 ──────────────────────────────────────────────────────────────────
[System.Windows.Forms.Application]::Run()
