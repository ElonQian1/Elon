<#
.SYNOPSIS
    LAN 分发客户端 / 守护进程（多项目、多产物共享单一后台进程）

.DESCRIPTION
    两种运行模式（由 -DaemonMode 参数决定）：

    【客户端模式（默认，供 publish-*.ps1 调用）】
        向注册目录写入产物配置，然后确保守护进程在后台运行。
        立即返回，不阻塞调用方。

    【守护进程模式（-DaemonMode，由客户端模式内部启动，无需手动调用）】
        - 监听端口 7788，对外提供 GET /dist/<project>/<artifact> 下载
        - 定期扫描注册目录，自动向各项目的服务器注册本机 LAN IP
        - 所有注册过期后自动退出

    ─────────── 注册文件格式 ───────────
    %TEMP%\lan-dist-registry\<project>_<artifact>.json:
    {
      "project_id":     "elon",
      "artifact_id":    "user-apk",
      "file_path":      "D:\\...\\ElonSpeed-latest.apk",
      "version_code":   42,
      "server_reg_url": "http://43.139.149.158:8080/app/lan-peer/register",
      "ttl_minutes":    120,
      "registered_at":  "2026-05-26T10:00:00.0000000Z"
    }

    ─────────── 守护进程提供的 URL 路径 ───────────
    GET /dist/elon/user-apk        → elon 用户端 APK
    GET /dist/elon/admin-apk       → elon 管理端 APK
    GET /dist/bb64a/user-apk       → bb64a 用户端 APK
    GET /dist/bb64a/windows-exe    → bb64a Windows 客户端

    ─────────── 新项目接入方式 ───────────
    在项目的 publish-*.ps1 末尾添加：
        $client = Join-Path $PSScriptRoot "lan-dist-client.ps1"
        & $client -ProjectId "myapp" -ArtifactId "user-apk" `
                  -FilePath $outputPath -VersionCode $newCode `
                  -ServerRegisterUrl "http://<your-server>/app/lan-peer/register"
    服务器端需支持 POST /app/lan-peer/register（接受 dist_path 字段）

.PARAMETER ProjectId
    项目标识符（如 "elon", "bb64a"）

.PARAMETER ArtifactId
    产物标识符（如 "user-apk", "admin-apk", "windows-exe"），默认 "apk"

.PARAMETER FilePath
    要对外分发的本地文件绝对路径

.PARAMETER VersionCode
    产物版本号（整数），用于注册到服务器和版本比对

.PARAMETER ServerRegisterUrl
    服务器 LAN peer 注册接口完整 URL
    示例：http://43.139.149.158:8080/app/lan-peer/register

.PARAMETER TtlMinutes
    本次注册有效期（分钟），默认 120

.PARAMETER DaemonMode
    守护进程模式（内部使用，不需要手动传）
#>
param(
    [string]$ProjectId,
    [string]$ArtifactId = "apk",
    [string]$FilePath,
    [int]$VersionCode,
    [string]$ServerRegisterUrl,
    [int]$TtlMinutes = 120,
    [switch]$DaemonMode
)

$RegistryDir  = "$env:TEMP\lan-dist-registry"
$PidFile      = "$env:TEMP\lan-dist-daemon.pid"
$LogFile      = "$env:TEMP\lan-dist-daemon.log"
$DaemonPort   = 7788
$DaemonScript = $MyInvocation.MyCommand.Path  # 自身路径，守护模式下会递归调用

# ─── 工具函数 ────────────────────────────────────────────────────────────────

function Write-Log {
    param([string]$Msg)
    $line = "[$(Get-Date -Format 'HH:mm:ss')] $Msg"
    Add-Content -Path $LogFile -Value $line -Encoding UTF8
}

function Get-LanIp {
    $candidates = [System.Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces() |
        Where-Object { $_.OperationalStatus -eq 'Up' -and $_.NetworkInterfaceType -ne 'Loopback' } |
        ForEach-Object {
            $_.GetIPProperties().UnicastAddresses |
            Where-Object { $_.Address.AddressFamily -eq 'InterNetwork' } |
            Select-Object -ExpandProperty Address
        } |
        Where-Object {
            $b = $_.GetAddressBytes()
            ($b[0] -eq 192 -and $b[1] -eq 168) -or
            ($b[0] -eq 10) -or
            ($b[0] -eq 172 -and $b[1] -ge 16 -and $b[1] -le 31)
        }
    # 优先 192.168.x.x
    $preferred = $candidates | Where-Object { $_.GetAddressBytes()[0] -eq 192 } | Select-Object -First 1
    if ($preferred) { return $preferred.ToString() }
    $firstCandidate = $candidates | Select-Object -First 1
    if ($firstCandidate) { return $firstCandidate.ToString() }
    return $null
}

function Get-ActiveEntries {
    $entries = @()
    $now = Get-Date
    Get-ChildItem "$RegistryDir\*.json" -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            $e = Get-Content $_.FullName -Raw -Encoding UTF8 | ConvertFrom-Json
            $age = $now - [datetime]$e.registered_at
            if ($age.TotalMinutes -lt $e.ttl_minutes -and (Test-Path $e.file_path)) {
                $entries += $e
            }
        } catch {}
    }
    return $entries
}

# ═══════════════════════════════════════════════════════════════════════════════
# 守护进程模式
# ═══════════════════════════════════════════════════════════════════════════════
if ($DaemonMode) {
    Write-Log "═══ LAN-Dist 守护进程启动 (port $DaemonPort) ═══"

    $LanIp = Get-LanIp
    if (-not $LanIp) {
        Write-Log "❌ 无法获取 LAN IP，退出"
        exit 1
    }
    Write-Log "🖥️  LAN IP: $LanIp"

    # TcpListener 不需要管理员权限（HttpListener 的 http://+:port/ 需要 URL ACL 或管理员）
    $TcpListener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Any, $DaemonPort)
    try {
        $TcpListener.Start()
        Write-Log "✅ HTTP 服务已启动，监听 :$DaemonPort"
    } catch {
        Write-Log "❌ 无法绑定端口 $DaemonPort : $_"
        exit 1
    }

    # 每个产物上次向服务器注册的时间
    $lastRegTime    = @{}
    $ReRegIntervalMin = 55   # 略小于服务器 2 小时 TTL

    function Invoke-ServerRegister {
        param($Entry)
        try {
            $urlPath = "/dist/$($Entry.project_id)/$($Entry.artifact_id)"
            $body = @{
                lan_ip       = $LanIp
                port         = $DaemonPort
                version_code = [int]$Entry.version_code
                dist_path    = $urlPath
            } | ConvertTo-Json -Compress
            Invoke-RestMethod -Method Post -Uri $Entry.server_reg_url `
                -Body $body -ContentType 'application/json' `
                -TimeoutSec 10 -NoProxy | Out-Null
            Write-Log "✅ 已注册 $($Entry.project_id)/$($Entry.artifact_id) → $urlPath  @$($Entry.server_reg_url)"
            return $true
        } catch {
            Write-Log "⚠️  注册失败 ($($Entry.project_id)/$($Entry.artifact_id)): $_"
            return $false
        }
    }

    function Send-HttpResponse {
        param($TcpClient, $StatusCode, $ContentType, [byte[]]$Bytes, $FileName)
        try {
            $TcpClient.ReceiveTimeout = 5000
            $TcpClient.SendTimeout   = 120000   # 大文件传输留足时间
            $ns  = $TcpClient.GetStream()
            $w   = [System.IO.StreamWriter]::new($ns, [System.Text.Encoding]::ASCII)
            $w.AutoFlush = $true
            $statusText  = if ($StatusCode -eq 200) { 'OK' } else { 'Not Found' }
            $w.WriteLine("HTTP/1.1 $StatusCode $statusText")
            $w.WriteLine("Content-Type: $ContentType")
            $w.WriteLine("Content-Length: $($Bytes.Length)")
            if ($FileName) { $w.WriteLine("Content-Disposition: attachment; filename=""$FileName""") }
            $w.WriteLine("Connection: close")
            $w.WriteLine("")
            $ns.Write($Bytes, 0, $Bytes.Length)
            $ns.Flush()
        } catch {}
        finally { try { $TcpClient.Close() } catch {} }
    }

    # ── 初次注册 ─────────────────────────────────────────────────────────────
    foreach ($e in (Get-ActiveEntries)) {
        $key = "$($e.project_id)_$($e.artifact_id)"
        if (Invoke-ServerRegister $e) { $lastRegTime[$key] = Get-Date }
    }

    $PollIntervalMs = 30000
    $LastPoll       = (Get-Date).AddMilliseconds(-$PollIntervalMs)
    $AcceptTask     = $TcpListener.AcceptTcpClientAsync()   # 挂起第一个 accept

    try {
        while ($true) {
            # 检查是否还有活跃条目
            if ((Get-ActiveEntries).Count -eq 0) {
                Write-Log "📭 所有产物已过期，守护进程退出"
                break
            }

            # 定期重新注册（防止服务器端 TTL 过期）
            $now = Get-Date
            if (($now - $LastPoll).TotalMilliseconds -ge $PollIntervalMs) {
                $LastPoll = $now
                foreach ($e in (Get-ActiveEntries)) {
                    $key     = "$($e.project_id)_$($e.artifact_id)"
                    $elapsed = if ($lastRegTime.ContainsKey($key)) {
                        ($now - $lastRegTime[$key]).TotalMinutes
                    } else { 9999 }
                    if ($elapsed -ge $ReRegIntervalMin) {
                        if (Invoke-ServerRegister $e) { $lastRegTime[$key] = $now }
                    }
                }
            }

            # 等待新连接（1 秒轮询；累计 ~30 秒做一次定期检查）
            if (-not $AcceptTask.Wait(1000)) { continue }

            $tcpClient  = $AcceptTask.Result
            $AcceptTask = $TcpListener.AcceptTcpClientAsync()   # 立即挂起下一个 accept

            try {
                $tcpClient.ReceiveTimeout = 5000
                $ns     = $tcpClient.GetStream()
                $reader = [System.IO.StreamReader]::new($ns, [System.Text.Encoding]::ASCII)

                # 读取 HTTP 请求行
                $requestLine = $reader.ReadLine()
                # 排空请求头（避免 socket 残留数据影响下次读取）
                while ($true) {
                    $hdr = $reader.ReadLine()
                    if ($null -eq $hdr -or $hdr -eq '') { break }
                }

                $remoteIp = $tcpClient.Client.RemoteEndPoint.Address

                if ($requestLine -and $requestLine -match '^GET\s+/dist/([^/\s]+)/([^/\s]+)') {
                    $proj  = $Matches[1]
                    $art   = $Matches[2]
                    $entry = Get-ActiveEntries |
                             Where-Object { $_.project_id -eq $proj -and $_.artifact_id -eq $art } |
                             Select-Object -First 1

                    if ($entry) {
                        $fname = Split-Path $entry.file_path -Leaf
                        $bytes = [System.IO.File]::ReadAllBytes($entry.file_path)
                        Send-HttpResponse $tcpClient 200 'application/octet-stream' $bytes $fname
                        $sizeMb = [math]::Round($bytes.Length / 1MB, 1)
                        Write-Log "served $proj/$art -> $remoteIp ($sizeMb MB)"
                    } else {
                        $err = [System.Text.Encoding]::UTF8.GetBytes('{"error":"artifact not found or expired"}')
                        Send-HttpResponse $tcpClient 404 'application/json' $err $null
                    }
                } else {
                    $err = [System.Text.Encoding]::UTF8.GetBytes('{"error":"not found"}')
                    Send-HttpResponse $tcpClient 404 'application/json' $err $null
                }
            } catch {
                Write-Log "⚠️  处理请求异常: $_"
                try { $tcpClient.Close() } catch {}
            }
        }
    } finally {
        $TcpListener.Stop()
        Remove-Item $PidFile -ErrorAction SilentlyContinue
        Write-Log "🛑 守护进程已停止"
    }
    exit 0
}

# ═══════════════════════════════════════════════════════════════════════════════
# 客户端模式（默认）
# ═══════════════════════════════════════════════════════════════════════════════

if (-not $ProjectId -or -not $FilePath -or $VersionCode -le 0 -or -not $ServerRegisterUrl) {
    Write-Error "客户端模式必须提供 -ProjectId, -FilePath, -VersionCode (>0), -ServerRegisterUrl"
    exit 1
}
if (-not (Test-Path $FilePath)) {
    Write-Error "文件不存在: $FilePath"
    exit 1
}

# 写注册文件
New-Item -ItemType Directory -Force -Path $RegistryDir | Out-Null
$entry = [ordered]@{
    project_id     = $ProjectId
    artifact_id    = $ArtifactId
    file_path      = (Resolve-Path $FilePath).Path
    version_code   = $VersionCode
    server_reg_url = $ServerRegisterUrl
    ttl_minutes    = $TtlMinutes
    registered_at  = (Get-Date).ToUniversalTime().ToString("o")
}
$entry | ConvertTo-Json | Set-Content "$RegistryDir\${ProjectId}_${ArtifactId}.json" -Encoding UTF8

# 确保守护进程在运行
$daemonRunning = $false
if (Test-Path $PidFile) {
    try {
        $savedPid = [int](Get-Content $PidFile -ErrorAction Stop)
        if ($savedPid -gt 0 -and (Get-Process -Id $savedPid -ErrorAction SilentlyContinue)) {
            $daemonRunning = $true
        }
    } catch {}
}

if (-not $daemonRunning) {
    # 首次启动：尝试添加 Windows 防火墙规则，允许手机通过 LAN 访问守护进程端口
    # 需要管理员权限；静默失败，仅打印手动命令供参考
    try {
        $ruleName = "LAN-Dist Port $DaemonPort"
        $chk = (netsh advfirewall firewall show rule name="$ruleName" 2>&1)
        if ($LASTEXITCODE -ne 0) {
            netsh advfirewall firewall add rule name="$ruleName" dir=in action=allow `
                protocol=TCP localport=$DaemonPort profile=private 2>&1 | Out-Null
            if ($LASTEXITCODE -eq 0) {
                Write-Host "   🔓 已添加防火墙规则（允许 LAN 端口 $DaemonPort 入站）" -ForegroundColor DarkGray
            } else {
                Write-Host "   ⚠️  防火墙规则未能自动添加（需管理员），如手机无法连接请手动运行：" -ForegroundColor DarkYellow
                Write-Host "       netsh advfirewall firewall add rule name=`"$ruleName`" dir=in action=allow protocol=TCP localport=$DaemonPort profile=private" -ForegroundColor DarkYellow
            }
        }
    } catch {}
    $proc = Start-Process pwsh -WindowStyle Hidden -PassThru -ArgumentList @(
        "-NonInteractive",
        "-ExecutionPolicy", "Bypass",
        "-File", $DaemonScript,
        "-DaemonMode"
    )
    $proc.Id | Set-Content $PidFile -Encoding UTF8
    Write-Host "   ✅ LAN-Dist 守护进程已启动 (PID: $($proc.Id))，注册 $ProjectId/$ArtifactId" -ForegroundColor Green
} else {
    Write-Host "   ✅ LAN-Dist 守护进程运行中 (PID: $savedPid)，已追加注册 $ProjectId/$ArtifactId" -ForegroundColor Green
}
Write-Host "   📡 下载地址: http://<LAN-IP>:$DaemonPort/dist/$ProjectId/$ArtifactId" -ForegroundColor DarkGray
Write-Host "   📄 守护日志: $LogFile" -ForegroundColor DarkGray
