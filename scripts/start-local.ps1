# start-local.ps1 — 在 Windows 本机以单用户模式启动 elon server
#
# 用途：让 APK 直连本机，无需通过云端，适合 PC 侧开发调试。
# 用法：scripts\start-local.ps1
#       如需覆盖 token：scripts\start-local.ps1 -OwnerToken "your64hextoken"
#
# APK 配置（AgentConfigActivity 备用服务器）：
#   备用服务器地址: http://192.168.31.142:7800
#   备用 Token:     (脚本启动时打印)

param(
    [string]$OwnerToken = "",
    [string]$Port = "7800"
)

Set-Location "$PSScriptRoot\.."

# ── 生成或使用 owner token ──────────────────────────────────────────────────
if (-not $OwnerToken) {
    # 优先读取本地持久化的 token（避免每次重启 APK 都要重新输入）
    $tokenFile = "$PSScriptRoot\..\data-local\.owner_token"
    if (Test-Path $tokenFile) {
        $OwnerToken = (Get-Content $tokenFile -Raw).Trim()
        Write-Host "[start-local] 使用已有 owner token（读自 $tokenFile）"
    } else {
        # 生成新的加密随机 token
        $bytes = [System.Security.Cryptography.RandomNumberGenerator]::GetBytes(32)
        $OwnerToken = ([System.BitConverter]::ToString($bytes) -replace '-','').ToLower()
        New-Item -ItemType Directory -Force -Path "$PSScriptRoot\..\data-local" | Out-Null
        $OwnerToken | Set-Content $tokenFile
        Write-Host "[start-local] 已生成新 owner token，已保存到 $tokenFile"
    }
}

# ── 显示连接信息 ────────────────────────────────────────────────────────────
$localIp = (Get-NetIPAddress -AddressFamily IPv4 -InterfaceAlias "WLAN*","Wi-Fi*","以太网*","Ethernet*" `
    -ErrorAction SilentlyContinue | Where-Object { $_.IPAddress -notlike "169.*" } |
    Select-Object -First 1).IPAddress
if (-not $localIp) { $localIp = "127.0.0.1" }

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  elon server 本地模式" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  本机 IP   : $localIp"
Write-Host "  监听地址  : 0.0.0.0:$Port"
Write-Host "  局域网URL : http://${localIp}:$Port         ← 同WiFi时 APK 填这个"
Write-Host "  互联网URL : http://43.139.149.158:8080/api/pc-relay/elon-pc-1  ← 跨网时 APK 填这个"
Write-Host "  Owner Token: $OwnerToken"
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# ── 设置环境变量 ────────────────────────────────────────────────────────────
$env:LISTEN_ADDR     = "0.0.0.0:$Port"
$env:OWNER_TOKEN     = $OwnerToken
$env:DATA_DIR        = "$PSScriptRoot\..\data-local"
$env:DATABASE_PATH   = "$PSScriptRoot\..\data-local\elon.db"
$env:WORKSPACE_ROOT  = "$PSScriptRoot\..\workspaces-local"
$env:PUBLIC_URL      = "http://${localIp}:$Port"
$env:ADMIN_TOKEN     = "local-admin-$(($OwnerToken).Substring(0,8))"
$env:REQUIRE_LOGIN   = "false"
$env:AI_BACKEND      = "local_cli"
$env:RUST_LOG        = "info,elon_server=debug"
$env:LOCAL_SERVER_PORT = $Port

# ── AI CLI 优先级：Copilot CLI → 回退 Codex CLI ──────────────────────────────
$env:AI_CODEX_CLI_ONLY     = "false"         # 同时加载 copilot + codex 两个选项
$env:COPILOT_CLI_ENABLED   = "true"
$env:COPILOT_CLI_ARGS      = "--allow-all-tools --allow-all-paths -p"  # 非交互模式
$env:AI_CLI_DEFAULT        = "copilot_cli"   # 优先使用 Copilot CLI
$env:AI_CLI_FALLBACK       = "codex_cli"     # Copilot 失败时回退到 Codex CLI

# PC → 云端 agent 反向代理配置（让 APK 通过 /api/pc-relay/{agent_id}/... 访问本机）
$cloudWs = "ws://43.139.149.158:8080/agent/ws"
$agentId = "elon-pc-1"
$agentSecretFile = "$PSScriptRoot\..\data-local\.agent_secret"
if (Test-Path $agentSecretFile) {
    $agentSecret = (Get-Content $agentSecretFile -Raw).Trim()
} else {
    $b = [System.Security.Cryptography.RandomNumberGenerator]::GetBytes(32)
    $agentSecret = ([System.BitConverter]::ToString($b) -replace '-','').ToLower()
    $agentSecret | Set-Content $agentSecretFile
    Write-Host "[start-local] 已生成新 agent secret，保存到 $agentSecretFile"
    Write-Host ""
    Write-Host "⚠️  首次启动：需要在服务器注册 agent secret！" -ForegroundColor Yellow
    Write-Host "   运行以下命令（需要 ADMIN_TOKEN）：" -ForegroundColor Yellow
    Write-Host "   ssh root@43.139.149.158 'echo ""ELON_AGENT_SECRETS=elon-pc-1:$agentSecret"" >> /etc/elon-server.env && systemctl restart elon-server'" -ForegroundColor Cyan
    Write-Host ""
}
$env:RELAY_CLOUD_URL  = $cloudWs
$env:ELON_AGENT_ID    = $agentId
$env:ELON_AGENT_SECRET = $agentSecret

# 如果有 GitHub Token，自动启用 Copilot API 代理
if ($env:GITHUB_TOKEN) {
    Write-Host "[start-local] 检测到 GITHUB_TOKEN，Copilot API 代理已启用"
} elseif ($env:COPILOT_GITHUB_TOKEN) {
    $env:GITHUB_TOKEN = $env:COPILOT_GITHUB_TOKEN
    Write-Host "[start-local] 检测到 COPILOT_GITHUB_TOKEN，已映射为 GITHUB_TOKEN"
}

# ── 确保数据目录存在 ────────────────────────────────────────────────────────
New-Item -ItemType Directory -Force -Path $env:DATA_DIR | Out-Null
New-Item -ItemType Directory -Force -Path $env:WORKSPACE_ROOT | Out-Null

# ── 启动服务 ────────────────────────────────────────────────────────────────
Write-Host "[start-local] 启动中... (Ctrl+C 停止)"
cargo run --manifest-path "$PSScriptRoot\..\server\Cargo.toml"
