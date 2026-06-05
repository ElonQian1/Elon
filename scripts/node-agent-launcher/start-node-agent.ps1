# 一龙 PC 节点启动器
# 逻辑：
#   1. 检测管理端口（默认 7799）是否已有进程监听
#   2. 已监听 → 节点在运行，直接打开管理页（浏览器单实例标签会自动跳回已开的标签）
#   3. 未监听 → 从 node-agent.env 加载配置 + 找 exe + 隐藏后台启动 → 等端口就绪 → 打开管理页
$ErrorActionPreference = 'Stop'

$here = Split-Path -Parent $MyInvocation.MyCommand.Path

function Load-EnvFile($path) {
    if (-not (Test-Path $path)) { return }
    foreach ($raw in Get-Content -LiteralPath $path) {
        $line = $raw.Trim()
        if (-not $line -or $line.StartsWith('#') -or -not $line.Contains('=')) { continue }
        $idx = $line.IndexOf('=')
        $key = $line.Substring(0, $idx).Trim()
        $val = $line.Substring($idx + 1).Trim()
        # 去掉成对引号
        if (($val.StartsWith('"') -and $val.EndsWith('"')) -or ($val.StartsWith("'") -and $val.EndsWith("'"))) {
            if ($val.Length -ge 2) { $val = $val.Substring(1, $val.Length - 2) }
        }
        if ($key) { [Environment]::SetEnvironmentVariable($key, $val, 'Process') }
    }
}

function Test-PortOpen($port) {
    try {
        $client = New-Object System.Net.Sockets.TcpClient
        $async = $client.BeginConnect('127.0.0.1', [int]$port, $null, $null)
        $ok = $async.AsyncWaitHandle.WaitOne(800)
        $connected = $ok -and $client.Connected
        $client.Close()
        return $connected
    } catch { return $false }
}

function Show-Msg($text) {
    try {
        Add-Type -AssemblyName System.Windows.Forms -ErrorAction SilentlyContinue
        [System.Windows.Forms.MessageBox]::Show($text, '一龙 PC 节点启动器') | Out-Null
    } catch {
        Write-Host $text
    }
}

# ── 先加载配置（端口可能被 NODE_ADMIN_PORT 覆盖）──────────────────────────────
$envFile = Join-Path $here 'node-agent.env'
Load-EnvFile $envFile

$port = $env:NODE_ADMIN_PORT
if (-not $port) { $port = '7799' }
$adminUrl = "http://127.0.0.1:$port/"

# ── 已在运行 → 直接打开管理页 ────────────────────────────────────────────────
if (Test-PortOpen $port) {
    Start-Process $adminUrl
    return
}

# ── 未运行 → 校验配置 + 找 exe + 启动 ────────────────────────────────────────
foreach ($req in 'NODE_AGENT_ID', 'NODE_AGENT_SECRET', 'NODE_OWNER_USER_ID') {
    if (-not [Environment]::GetEnvironmentVariable($req, 'Process')) {
        Show-Msg "缺少必填配置：$req`n`n请编辑同目录的 node-agent.env 后重试。`n（首次使用：把 node-agent.env.example 复制为 node-agent.env 并填写）"
        return
    }
}

$exe = $env:NODE_AGENT_EXE
if (-not $exe -or -not (Test-Path $exe)) {
    $exe = Join-Path $here 'elon-node-agent.exe'
}
if (-not (Test-Path $exe)) {
    Show-Msg "找不到 elon-node-agent.exe。`n`n请把它放到：`n$here`n`n或在 node-agent.env 里设置：`nNODE_AGENT_EXE=完整路径"
    return
}

# 隐藏后台启动节点进程
Start-Process -FilePath $exe -WindowStyle Hidden -WorkingDirectory $here | Out-Null

# 等端口就绪（最多 ~15 秒）
$ready = $false
for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Milliseconds 500
    if (Test-PortOpen $port) { $ready = $true; break }
}

if (-not $ready) {
    Show-Msg "节点已启动，但管理页端口 $port 在 15 秒内未就绪。`n稍后可手动打开：$adminUrl"
    return
}

Start-Process $adminUrl
