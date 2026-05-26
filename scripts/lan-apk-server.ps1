<#
.SYNOPSIS
    [兼容 shim] 局域网 APK 种子服务入口 — 已由 lan-dist-client.ps1 统一接管

.DESCRIPTION
    此脚本保留仅为向后兼容（以防其他地方引用了旧名称）。
    实际逻辑已迁移到 lan-dist-client.ps1，支持多项目、多产物（APK/EXE等）共享守护进程。
    新项目请直接调用 lan-dist-client.ps1。

.PARAMETER ApkPath
    要提供下载的 APK 文件完整路径

.PARAMETER VersionCode
    该 APK 的 versionCode（用于向服务器注册）

.PARAMETER ServerUrl
    远端服务器地址（如 http://43.139.149.158:8080）

.PARAMETER ProjectId
    项目标识符（默认 "elon"）

.PARAMETER ArtifactId
    产物标识符（默认 "user-apk"）

.PARAMETER TtlMinutes
    有效期分钟数（默认 120）

.PARAMETER Port
    本地监听端口，默认 7788

.PARAMETER TtlMinutes
    服务运行时间（分钟），默认 120 分钟（2 小时）

.NOTES
    本脚本通常由 publish-apk.ps1 以 -WindowStyle Hidden 方式在后台启动，
    不应出现在用户桌面。日志写到 %TEMP%\lan-dist-daemon.log。
#>
param(
    [Parameter(Mandatory = $true)] [string]$ApkPath,
    [Parameter(Mandatory = $true)] [int]$VersionCode,
    [Parameter(Mandatory = $true)] [string]$ServerUrl,
    [string]$ProjectId  = "elon",
    [string]$ArtifactId = "user-apk",
    [int]$Port       = 7788,
    [int]$TtlMinutes = 120
)

# ── 兼容 shim：直接委托给 lan-dist-client.ps1 ──────────────────────────────
$client = Join-Path $PSScriptRoot "lan-dist-client.ps1"
if (-not (Test-Path $client)) {
    Write-Error "未找到 $client，请确认 lan-dist-client.ps1 存在于同目录"
    exit 1
}
& $client `
    -ProjectId         $ProjectId `
    -ArtifactId        $ArtifactId `
    -FilePath          $ApkPath `
    -VersionCode       $VersionCode `
    -ServerRegisterUrl "$($ServerUrl.TrimEnd('/'))/app/lan-peer/register" `
    -TtlMinutes        $TtlMinutes
