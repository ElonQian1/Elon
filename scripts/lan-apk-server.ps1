<#
.SYNOPSIS
    局域网 APK 文件服务器（后台无窗口运行）

.DESCRIPTION
    供 publish-apk.ps1 在发布成功后调用，在本机启动一个 HTTP 文件服务器：
      - 监听 0.0.0.0:7788，对外提供 GET /apk 下载 APK 文件
      - 启动后向远端服务器注册本机 LAN IP，服务器将其注入 version.json mirrors
      - 手机 APK 收到 version.json 后优先从本机 LAN 下载（速度更快）
      - 2 小时后自动退出，或者当服务器已有更新版本时提前退出

.PARAMETER ApkPath
    要提供下载的 APK 文件完整路径

.PARAMETER VersionCode
    该 APK 的 versionCode（用于向服务器注册）

.PARAMETER ServerUrl
    远端服务器地址（如 http://43.139.149.158:8080）

.PARAMETER Port
    本地监听端口，默认 7788

.PARAMETER TtlMinutes
    服务运行时间（分钟），默认 120 分钟（2 小时）

.NOTES
    本脚本通常由 publish-apk.ps1 以 -WindowStyle Hidden 方式在后台启动，
    不应出现在用户桌面。日志写到 %TEMP%\elon-lan-apk-server.log。
#>
param(
    [Parameter(Mandatory = $true)] [string]$ApkPath,
    [Parameter(Mandatory = $true)] [int]$VersionCode,
    [Parameter(Mandatory = $true)] [string]$ServerUrl,
    [int]$Port = 7788,
    [int]$TtlMinutes = 120
)

$LogFile = "$env:TEMP\elon-lan-apk-server.log"
$ServerUrl = $ServerUrl.TrimEnd('/')

function Write-Log {
    param([string]$Msg)
    $line = "[$(Get-Date -Format 'HH:mm:ss')] $Msg"
    Add-Content -Path $LogFile -Value $line -Encoding UTF8
}

# ── 获取本机局域网 IP ────────────────────────────────────────────────────────

function Get-LanIp {
    # 取第一个私有 IPv4 地址（优先 192.168.x.x，其次 10.x.x.x）
    $candidates = [System.Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces() |
        Where-Object { $_.OperationalStatus -eq 'Up' -and $_.NetworkInterfaceType -ne 'Loopback' } |
        ForEach-Object {
            $_.GetIPProperties().UnicastAddresses |
            Where-Object { $_.Address.AddressFamily -eq 'InterNetwork' } |
            Select-Object -ExpandProperty Address
        } |
        Where-Object {
            $bytes = $_.GetAddressBytes()
            ($bytes[0] -eq 192 -and $bytes[1] -eq 168) -or
            ($bytes[0] -eq 10) -or
            ($bytes[0] -eq 172 -and $bytes[1] -ge 16 -and $bytes[1] -le 31)
        }

    # 192.168.x.x 优先
    $preferred = $candidates | Where-Object { $_.GetAddressBytes()[0] -eq 192 } | Select-Object -First 1
    if ($preferred) { return $preferred.ToString() }
    return ($candidates | Select-Object -First 1)?.ToString()
}

$LanIp = Get-LanIp
if (-not $LanIp) {
    Write-Log "❌ 无法获取局域网 IP，退出"
    exit 1
}
Write-Log "🖥️  LAN IP: $LanIp  端口: $Port  APK: $ApkPath  versionCode: $VersionCode"

# ── 向服务器注册 ─────────────────────────────────────────────────────────────

function Register-LanPeer {
    try {
        $body = @{ lan_ip = $LanIp; port = $Port; version_code = $VersionCode } | ConvertTo-Json
        $resp = Invoke-RestMethod -Method Post `
            -Uri "$ServerUrl/app/lan-peer/register" `
            -Body $body -ContentType 'application/json' `
            -TimeoutSec 10 -NoProxy
        Write-Log "✅ 已注册为 LAN 种子: $($resp.peer_id)（有效期 $($resp.expires_in)s）"
        return $true
    } catch {
        Write-Log "⚠️  注册失败: $_"
        return $false
    }
}

$registered = Register-LanPeer
if (-not $registered) {
    Write-Log "注册失败，但仍继续提供局域网服务（手机可手动连接）"
}

# ── 启动 HTTP 文件服务器 ─────────────────────────────────────────────────────

if (-not (Test-Path $ApkPath)) {
    Write-Log "❌ APK 文件不存在: $ApkPath"
    exit 1
}

$Listener = [System.Net.HttpListener]::new()
$Listener.Prefixes.Add("http://+:$Port/")

try {
    $Listener.Start()
    Write-Log "🚀 HTTP 服务已启动，监听 http://${LanIp}:$Port/apk"
} catch {
    Write-Log "❌ 无法启动监听（端口 $Port 可能被占用）: $_"
    exit 1
}

$Deadline = (Get-Date).AddMinutes($TtlMinutes)
$LastVersionCheck = (Get-Date).AddMinutes(-10) # 立即首次检查

try {
    while ((Get-Date) -lt $Deadline) {
        # 定期检查服务器是否已有更新版本（每 10 分钟）
        if ((Get-Date) - $LastVersionCheck -gt [TimeSpan]::FromMinutes(10)) {
            $LastVersionCheck = Get-Date
            try {
                $vj = Invoke-RestMethod -Uri "$ServerUrl/app/version.json" -TimeoutSec 8 -NoProxy
                if ($vj.versionCode -gt $VersionCode) {
                    Write-Log "🆕 服务器已有更新版本 build $($vj.versionCode) > $VersionCode，本服务退出"
                    break
                }
            } catch {
                # 版本检查失败，继续服务
            }
        }

        # 等待请求（最多 30 秒超时，以便定期检查版本和到期时间）
        $asyncResult = $Listener.BeginGetContext($null, $null)
        $waited = $asyncResult.AsyncWaitHandle.WaitOne(30000)
        if (-not $waited) { continue }

        $ctx = $Listener.EndGetContext($asyncResult)
        $req = $ctx.Request
        $resp = $ctx.Response

        try {
            if ($req.HttpMethod -eq 'GET' -and $req.Url.AbsolutePath -in @('/apk', '/apk/')) {
                $apkBytes = [System.IO.File]::ReadAllBytes($ApkPath)
                $resp.ContentType = 'application/vnd.android.package-archive'
                $resp.ContentLength64 = $apkBytes.Length
                $resp.AddHeader('Content-Disposition', 'attachment; filename="ElonSpeed-latest.apk"')
                $resp.OutputStream.Write($apkBytes, 0, $apkBytes.Length)
                Write-Log "📤 已传输 APK 给 $($req.RemoteEndPoint.Address) ($([math]::Round($apkBytes.Length/1MB, 1)) MB)"
            } else {
                $resp.StatusCode = 404
                $notFound = [System.Text.Encoding]::UTF8.GetBytes('{"error":"not found"}')
                $resp.ContentType = 'application/json'
                $resp.OutputStream.Write($notFound, 0, $notFound.Length)
            }
        } catch {
            Write-Log "⚠️  处理请求异常: $_"
        } finally {
            $resp.Close()
        }
    }
} finally {
    $Listener.Stop()
    Write-Log "🛑 LAN APK 服务已停止"
}
