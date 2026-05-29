<#
.SYNOPSIS
    在本地 Windows 开发机启动 elon-server，作为云端服务器的备用 (fallback)。

.DESCRIPTION
    - 编译 elon-server（增量构建，第一次耗时约 5-10 分钟）
    - 在本地端口 8081 启动，避免与其他服务冲突
    - APK 设置页面把「备用服务器地址」填写为本机 Tailscale IP + 端口:8081

.NOTES
    前置条件:
      1. 已安装 Rust (rustup)
      2. 本机已安装 Tailscale (推荐) 或与手机同 WiFi
      3. 先配置好下方 $env: 变量，尤其是 ADMIN_TOKEN 和 AI 接入配置

    APK 配置步骤:
      1. 获取本机 Tailscale IP: tailscale ip -4
      2. 在 APK「代理配置」→「备用服务器地址」填写: http://<tailscale-ip>:8081
      3. 主服务器断开 3 次后，APK 自动切换到本地服务器
#>

param(
    [switch]$NoBuild,          # 跳过编译，直接运行已有的 release binary
    [switch]$Debug,            # 用 debug 模式编译（更快，性能较低）
    [string]$Port = "8081"     # 监听端口
)

$ErrorActionPreference = "Stop"

# ── 路径配置 ──────────────────────────────────────────────────────────────────
$repoRoot  = "D:\rust\active-projects\elon cli"
$serverDir = Join-Path $repoRoot "server"
$dataDir   = Join-Path $repoRoot "local-data"
$apkDir    = Join-Path $dataDir "app"
$logFile   = Join-Path $dataDir "elon-server-local.log"

# ── 确保数据目录存在 ──────────────────────────────────────────────────────────
New-Item -ItemType Directory -Force -Path $dataDir | Out-Null
New-Item -ItemType Directory -Force -Path $apkDir  | Out-Null

# ── 获取本机 IP（优先 Tailscale，回退到局域网 IP）────────────────────────────
function Get-LocalIP {
    # 先尝试 Tailscale
    try {
        $tsIp = & tailscale ip -4 2>$null
        if ($tsIp -match '100\.\d+\.\d+\.\d+') { return $tsIp.Trim() }
    } catch {}
    # 回退到第一个非回环 IPv4
    $ip = (Get-NetIPAddress -AddressFamily IPv4 |
           Where-Object { $_.InterfaceAlias -notmatch 'Loopback|Loopback' -and $_.IPAddress -ne '127.0.0.1' } |
           Select-Object -First 1).IPAddress
    return $ip
}

$localIp = Get-LocalIP
$publicUrl = "http://${localIp}:${Port}"

Write-Host "本机 IP: $localIp"
Write-Host "备用服务器地址（填入 APK 设置）: $publicUrl"

# ── 环境变量配置 ───────────────────────────────────────────────────────────────
# ⚠️ 安全提示：以下 TOKEN/KEY 不要提交到 git；可以将真实值写到本地 .env.local 文件
#    然后在此 source：if (Test-Path ".env.local") { Get-Content ".env.local" | ForEach-Object { if ($_ -match "^(\w+)=(.*)") { Set-Item "env:$($Matches[1])" $Matches[2] } } }

$env:LISTEN_ADDR     = "0.0.0.0:$Port"
$env:DATA_DIR        = $dataDir
$env:DATABASE_PATH   = Join-Path $dataDir "elon.db"
$env:PUBLIC_URL      = $publicUrl
$env:ELON_SELF_PATH  = $repoRoot                      # elon 自项目根目录

# 管理员令牌（必须改，默认值不安全）
if (-not $env:ADMIN_TOKEN) {
    $env:ADMIN_TOKEN = "elon-local-admin"
    Write-Warning "ADMIN_TOKEN 未设置，使用默认值 'elon-local-admin'，本地调试可接受"
}

# AI LLM 配置（与云端保持一致；如果已设置则跳过）
# 优先读取用户环境变量；需要时手动设置：
#   $env:AGENT_MAIN_KEY   = "your-api-key"
#   $env:AGENT_MAIN_BASE  = "https://api.hunyuan.cloud.tencent.com/v1"  # 混元
#   $env:AGENT_MAIN_MODEL = "hunyuan-turbo"

# Whisper 语音识别（本地先用云端服务；如需本地 Whisper 见下方注释）
# 云端 Whisper：无需配置，使用云端 43.139.149.158:8080 的 ASR 端点
# 本地 Whisper：需要启动 whisper_service.py（需要 Python + faster-whisper）
#   $env:WHISPER_URL = "http://127.0.0.1:5001/asr"

# homecli 反向通道（如果本机也运行 homecli agent 连接本地服务器）
if (-not $env:ELON_AGENT_SECRETS) {
    $env:ELON_AGENT_SECRETS = ""  # 格式: "agent-id:secret32chars"
}

# ── 编译 ──────────────────────────────────────────────────────────────────────
if (-not $NoBuild) {
    Push-Location $serverDir
    try {
        $buildMode = if ($Debug) { "debug" } else { "release" }
        Write-Host "`n编译 elon-server ($buildMode)..." -ForegroundColor Cyan

        # 注意：全局 target 目录在 D:\rust\shared\target（.cargo/config.toml 配置）
        # 发布脚本屏蔽 native 优化，本地运行无限制
        if ($Debug) {
            cargo build -p server 2>&1
        } else {
            cargo build -p server --release 2>&1
        }
        if ($LASTEXITCODE -ne 0) { throw "编译失败！" }
        Write-Host "编译成功" -ForegroundColor Green
    } finally {
        Pop-Location
    }
}

# ── 确定 binary 路径 ──────────────────────────────────────────────────────────
$sharedTarget = "D:\rust\shared\target"
if ($Debug) {
    $binary = Join-Path $sharedTarget "debug\server.exe"
} else {
    $binary = Join-Path $sharedTarget "release\server.exe"
}

if (-not (Test-Path $binary)) {
    Write-Error "找不到 binary: $binary`n请先运行 scripts\start-local-elon-server.ps1（不加 -NoBuild）"
    exit 1
}

# ── 启动服务器 ────────────────────────────────────────────────────────────────
Write-Host "`n启动本地 elon-server..." -ForegroundColor Green
Write-Host "  监听地址: $($env:LISTEN_ADDR)"
Write-Host "  数据目录: $($env:DATA_DIR)"
Write-Host "  公开 URL: $($env:PUBLIC_URL)"
Write-Host "  日志文件: $logFile"
Write-Host ""
Write-Host "APK 设置 → 代理配置 → 备用服务器地址: $publicUrl" -ForegroundColor Yellow
Write-Host "按 Ctrl+C 停止服务器`n" -ForegroundColor DarkGray

# 同时输出到终端和日志文件
& $binary 2>&1 | Tee-Object -FilePath $logFile
