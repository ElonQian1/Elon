<#
.SYNOPSIS
    一龙 Android APK 发布脚本（版本号由服务器分配，build.gradle 不再进 git）

.DESCRIPTION
    新版业务流程（version-from-server）：
      1. git fetch origin main + fast-forward only (业务 commit 必须先由 AI 自己 push)
      2. POST /api/release/claim            (服务器原子分配 assignedVersionName + assignedVersionCode)
      3. 临时改写 build.gradle 注入版本号 → ./gradlew assembleRelease
      4. 上传 APK + version.json (CAS by .apk-deployed-sha)
      5. POST /api/release/finish           (释放 in-flight 槽位)
      6. **不 commit build.gradle** —— 还原文件，git 里 build.gradle 版本号永远是冷启动兜底。

    版本号不再进入 git 历史；多 AI / 多 PC 不会因为撞版本号死循环 rebase。

.PARAMETER Changelog
    本次版本更新说明（必填，写进 version.json 给手机端用户看）

.PARAMETER SkipBuild
    跳过 Gradle 编译，直接用已有的 APK 重新上传（用于调试脚本）

.PARAMETER Force
    跳过上传前的"服务器已有更新 versionCode"检查，强制覆盖。

.EXAMPLE
    .\publish-apk.ps1 -Changelog "修复启动闪退"
    .\publish-apk.ps1 -SkipBuild -Changelog "仅重新上传"
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$Changelog,

    [switch]$SkipBuild,

    # 跳过上传前的"服务器已发布更新 versionCode"检查，强制覆盖
    [switch]$Force,

    # 恢复用：当线上 version.json 曾回退，但用户手机已安装更高 build 时，
    # 以该已安装 build 作为 claim 最低基线，仍由服务器分配下一个版本号。
    [int]$CurrentInstalledVersionCode = 0,

    [string]$CurrentInstalledVersionName = ''
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "direct-network.ps1")

Set-ElonProjectDirectNetwork

# ── Release API helper（同 publish-server.ps1） ─────────────────────────────
$ReleaseApiBase = "$($null)"
$ReleaseApiBase = "http://43.139.149.158:8080/api/release"

function Invoke-ReleaseApi {
    param(
        [Parameter(Mandatory)] [string]$Endpoint,
        [object]$Body = $null,
        [int]$TimeoutSec = 20
    )
    $url = "$ReleaseApiBase/$Endpoint"
    $tmp = $null
    try {
        $curlArgs = @('--noproxy','*','-s','--max-time',$TimeoutSec,'-w','\n__HTTP_STATUS__:%{http_code}','-X','POST',$url)
        if ($Body) {
            $json = ($Body | ConvertTo-Json -Depth 6 -Compress)
            $tmp = [System.IO.Path]::GetTempFileName()
            [System.IO.File]::WriteAllText($tmp, $json, [System.Text.UTF8Encoding]::new($false))
            $curlArgs += @('-H','Content-Type: application/json; charset=utf-8','--data-binary',"@$tmp")
        }
        $raw = & curl.exe @curlArgs 2>&1
        $rawText = ($raw -join "`n")
        if ($LASTEXITCODE -ne 0) {
            throw "curl 调用失败 ($Endpoint, exit=$LASTEXITCODE): $rawText"
        }
        $statusLine = ($rawText -split "`n") | Where-Object { $_ -match '^__HTTP_STATUS__:' } | Select-Object -Last 1
        $bodyText = ($rawText -replace "(?s)\n?__HTTP_STATUS__:\d+\s*$","")
        $status = if ($statusLine) { [int]($statusLine -replace '^__HTTP_STATUS__:','') } else { 0 }
        if ($status -lt 200 -or $status -ge 300) {
            throw "release/$Endpoint HTTP ${status}: $bodyText"
        }
        if ([string]::IsNullOrWhiteSpace($bodyText)) { return $null }
        return ($bodyText | ConvertFrom-Json)
    } finally {
        if ($tmp -and (Test-Path $tmp)) { Remove-Item $tmp -Force -ErrorAction SilentlyContinue }
    }
}

function Invoke-HttpJson {
    param(
        [Parameter(Mandatory)] [string]$Url,
        [int]$TimeoutSec = 10
    )
    $raw = & curl.exe --noproxy '*' -s --max-time $TimeoutSec -w "`n__HTTP_STATUS__:%{http_code}" $Url 2>&1
    $rawText = ($raw -join "`n")
    if ($LASTEXITCODE -ne 0) {
        throw "curl GET 失败 (exit=$LASTEXITCODE): $rawText"
    }
    $statusLine = ($rawText -split "`n") | Where-Object { $_ -match '^__HTTP_STATUS__:' } | Select-Object -Last 1
    $bodyText = ($rawText -replace "(?s)\n?__HTTP_STATUS__:\d+\s*$","")
    $status = if ($statusLine) { [int]($statusLine -replace '^__HTTP_STATUS__:','') } else { 0 }
    if ($status -lt 200 -or $status -ge 300) {
        throw "HTTP ${status}: $bodyText"
    }
    if ([string]::IsNullOrWhiteSpace($bodyText)) { return $null }
    return ($bodyText | ConvertFrom-Json)
}

function Get-ReleaseStatus {
    param([Parameter(Mandatory)] [string]$Kind)
    Invoke-HttpJson -Url "$ReleaseApiBase/status?kind=$Kind" -TimeoutSec 10
}

$script:ReleaseToken = $null
$script:ReleaseFinished = $false

function Complete-Release {
    param(
        [Parameter(Mandatory)] [bool]$Success,
        [string]$VersionName = '',
        [int]$VersionCode = 0,
        [string]$Sha = '',
        [string]$ErrorMessage = ''
    )
    if (-not $script:ReleaseToken -or $script:ReleaseFinished) { return }
    try {
        $payload = @{
            kind  = 'apk'
            token = $script:ReleaseToken
            success = $Success
        }
        if ($Success) {
            if ($VersionName) { $payload.versionName = $VersionName }
            if ($VersionCode -gt 0) { $payload.versionCode = $VersionCode }
            if ($Sha)         { $payload.sha = $Sha }
        } else {
            if ($ErrorMessage) { $payload.errorMessage = $ErrorMessage }
        }
        Invoke-ReleaseApi -Endpoint 'finish' -Body $payload | Out-Null
        $script:ReleaseFinished = $true
    } catch {
        Write-Host "   ⚠️  release/finish 调用失败（不影响主流程）: $_" -ForegroundColor Yellow
    }
}

# 全局错误兜底：任何未捕获的 terminating error 也释放槽位
trap {
    try {
        if (Get-Command Restore-GradleVersionFile -ErrorAction SilentlyContinue) {
            Restore-GradleVersionFile
        }
    } catch {}
    try {
        if ($script:ReleaseToken -and -not $script:ReleaseFinished) {
            Complete-Release -Success:$false -ErrorMessage ("uncaught error: " + ($_ | Out-String))
        }
    } catch {}
    # 让原始错误继续终止脚本；发布脚本不能在构建失败后继续上传旧 APK。
    break
}

# ── 路径配置 ──────────────────────────────────────────────────────────────────

$RepoRoot   = git -C $PSScriptRoot rev-parse --show-toplevel
$AndroidDir = Join-Path $RepoRoot "android"
$GradlePath = Join-Path $AndroidDir "app\build.gradle"
$ApkPattern = Join-Path $AndroidDir "app\build\outputs\apk\release\*.apk"

$ServerHost = "root@43.139.149.158"
$ServerDir  = "/opt/elon/data/app"
$ServerUrl  = "http://43.139.149.158:8080"
$ApkShaFile = "$ServerDir/.apk-deployed-sha"

$DefaultKeystore = Join-Path $env:USERPROFILE ".elon\signing\elon-release.jks"
$LegacyKeystore  = Join-Path $AndroidDir "app\elon-release.jks"
$UserGradleProps = Join-Path $env:USERPROFILE ".gradle\gradle.properties"
$OriginalGradleContent = $null
$BuildBaseSha = $null
$LocalHeadSha = $null

Push-Location $RepoRoot
try {
    Set-ElonProjectDirectGitSsh
} finally {
    Pop-Location
}

function Get-ServerApkVersionBaseline {
    $candidates = @()
    $errors = @()

    try {
        $status = Get-ReleaseStatus -Kind 'apk'
        $code = [int]$status.lastPublishedVersionCode
        $name = [string]$status.lastPublishedVersionName
        if ($code -gt 0 -and -not [string]::IsNullOrWhiteSpace($name)) {
            $candidates += [pscustomobject]@{
                Source      = '/api/release/status'
                VersionCode = $code
                VersionName = $name
            }
        }
    } catch {
        $errors += "/api/release/status?kind=apk: $_"
    }

    try {
        $deployed = Invoke-HttpJson -Url "$ServerUrl/app/version.json" -TimeoutSec 10
        $code = [int]$deployed.versionCode
        $name = [string]$deployed.versionName
        if ($code -gt 0 -and -not [string]::IsNullOrWhiteSpace($name)) {
            $candidates += [pscustomobject]@{
                Source      = '/app/version.json'
                VersionCode = $code
                VersionName = $name
            }
        }
    } catch {
        $errors += "/app/version.json: $_"
    }

    if ($candidates.Count -eq 0) {
        foreach ($errorText in $errors) {
            Write-Warning "   ⚠️  APK 版本基线读取失败：$errorText"
        }
        Write-Error "❌ 无法读取服务器 APK 版本基线；发布已停止，避免用 build.gradle 兜底版本发布。"
    }

    $selected = $candidates | Sort-Object VersionCode -Descending | Select-Object -First 1
    foreach ($candidate in $candidates) {
        if ($candidate.VersionCode -ne $selected.VersionCode -or $candidate.VersionName -ne $selected.VersionName) {
            Write-Warning "   ⚠️  服务器 APK 版本来源不一致：$($candidate.Source)=v$($candidate.VersionName) build $($candidate.VersionCode)，最终采用最高 build $($selected.VersionCode)"
        }
    }
    Write-Host "   ℹ️  服务器 APK 版本基线: v$($selected.VersionName) (build $($selected.VersionCode)) [$($selected.Source)]" -ForegroundColor DarkGray
    return $selected
}

function Get-AaptExecutable {
    $candidatePaths = @()
    $sdkRoots = @(
        $env:ANDROID_HOME,
        $env:ANDROID_SDK_ROOT,
        $(if ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA "Android\Sdk" } else { $null })
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -Unique

    foreach ($sdkRoot in $sdkRoots) {
        $buildTools = Join-Path $sdkRoot "build-tools"
        if (Test-Path -LiteralPath $buildTools) {
            $candidatePaths += Get-ChildItem -LiteralPath $buildTools -Directory -ErrorAction SilentlyContinue |
                Sort-Object Name -Descending |
                ForEach-Object { Join-Path $_.FullName "aapt.exe" }
            $candidatePaths += Get-ChildItem -LiteralPath $buildTools -Directory -ErrorAction SilentlyContinue |
                Sort-Object Name -Descending |
                ForEach-Object { Join-Path $_.FullName "aapt" }
        }
    }

    $pathAapt = Get-Command aapt.exe,aapt -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($pathAapt) { $candidatePaths += $pathAapt.Source }

    foreach ($candidate in ($candidatePaths | Where-Object { $_ } | Select-Object -Unique)) {
        if (Test-Path -LiteralPath $candidate) { return $candidate }
    }

    throw "未找到 aapt，无法校验 APK manifest 版本。请确认 Android SDK build-tools 已安装。"
}

function Get-ApkManifestVersion {
    param([Parameter(Mandatory)] [string]$ApkPath)

    if (-not (Test-Path -LiteralPath $ApkPath)) {
        throw "APK 文件不存在，无法校验版本: $ApkPath"
    }

    $aapt = Get-AaptExecutable
    $inspectPath = $ApkPath
    $tmpInspectApk = $null
    try {
        # Windows aapt can fail on non-ASCII workspace paths; inspect an ASCII temp copy.
        $tmpInspectApk = Join-Path $env:TEMP ("elon-aapt-inspect-" + [Guid]::NewGuid().ToString("N") + ".apk")
        Copy-Item -LiteralPath $ApkPath -Destination $tmpInspectApk -Force
        $inspectPath = $tmpInspectApk

        $badging = & $aapt dump badging $inspectPath 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "aapt dump badging 失败: $($badging -join "`n")"
        }
    } finally {
        if ($tmpInspectApk) {
            Remove-Item -LiteralPath $tmpInspectApk -Force -ErrorAction SilentlyContinue
        }
    }

    $packageLine = ($badging | Where-Object { $_ -match "^package:" } | Select-Object -First 1)
    if (-not $packageLine) {
        throw "aapt 输出中未找到 package 行，无法校验 APK 版本。"
    }

    $codeMatch = [regex]::Match($packageLine, "versionCode='([^']+)'")
    $nameMatch = [regex]::Match($packageLine, "versionName='([^']*)'")
    if (-not $codeMatch.Success -or -not $nameMatch.Success) {
        throw "aapt package 行缺少 versionCode/versionName: $packageLine"
    }

    [PSCustomObject]@{
        VersionCode = [int64]$codeMatch.Groups[1].Value
        VersionName = [string]$nameMatch.Groups[1].Value
    }
}

function Assert-ApkManifestVersion {
    param(
        [Parameter(Mandatory)] [string]$ApkPath,
        [Parameter(Mandatory)] [int]$ExpectedVersionCode,
        [Parameter(Mandatory)] [string]$ExpectedVersionName,
        [string]$Label = "APK"
    )

    $actual = Get-ApkManifestVersion -ApkPath $ApkPath
    if ($actual.VersionCode -ne $ExpectedVersionCode -or $actual.VersionName -ne $ExpectedVersionName) {
        throw "$Label manifest 版本不匹配：期望 v$ExpectedVersionName (build $ExpectedVersionCode)，实际 v$($actual.VersionName) (build $($actual.VersionCode))。已停止发布，避免手机端重复更新。"
    }

    Write-Host "   ✅ $Label manifest: v$($actual.VersionName) (build $($actual.VersionCode))" -ForegroundColor Green
}

function Assert-RemoteApkManifestVersion {
    param(
        [Parameter(Mandatory)] [int]$ExpectedVersionCode,
        [Parameter(Mandatory)] [string]$ExpectedVersionName
    )

    $tmpApk = Join-Path $env:TEMP ("elon-remote-apk-" + [Guid]::NewGuid().ToString("N") + ".apk")
    try {
        & curl.exe --noproxy '*' -f -L -sS --max-time 120 -o $tmpApk "$ServerUrl/app/ElonSpeed-latest.apk" 2>&1 | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "下载线上 APK 校验包体失败，curl exit=$LASTEXITCODE"
        }
        Assert-ApkManifestVersion -ApkPath $tmpApk -ExpectedVersionCode $ExpectedVersionCode -ExpectedVersionName $ExpectedVersionName -Label "线上 APK"
    } finally {
        Remove-Item -LiteralPath $tmpApk -Force -ErrorAction SilentlyContinue
    }
}

function Get-UserGradleProperty {
    param([string]$Name)

    if (-not (Test-Path $UserGradleProps)) { return $null }
    foreach ($line in Get-Content $UserGradleProps -Encoding UTF8) {
        $trimmed = $line.Trim()
        if ($trimmed.StartsWith("#") -or -not $trimmed.Contains("=")) { continue }
        $parts = $trimmed.Split("=", 2)
        if ($parts[0].Trim() -eq $Name) {
            return $parts[1].Trim().Trim('"')
        }
    }
    return $null
}

function Set-EnvFromUserGradleProperty {
    param([string]$Name)

    if (-not [string]::IsNullOrWhiteSpace((Get-Item "Env:$Name" -ErrorAction SilentlyContinue).Value)) {
        return
    }
    $value = Get-UserGradleProperty $Name
    if (-not [string]::IsNullOrWhiteSpace($value)) {
        Set-Item "Env:$Name" $value
    }
}

function Use-ReleaseSigningConfig {
    Set-EnvFromUserGradleProperty "ELON_RELEASE_KEYSTORE"
    Set-EnvFromUserGradleProperty "ELON_RELEASE_STORE_PASSWORD"
    Set-EnvFromUserGradleProperty "ELON_RELEASE_KEY_ALIAS"
    Set-EnvFromUserGradleProperty "ELON_RELEASE_KEY_PASSWORD"

    if ([string]::IsNullOrWhiteSpace($env:ELON_RELEASE_KEYSTORE)) {
        if (Test-Path $DefaultKeystore) {
            $env:ELON_RELEASE_KEYSTORE = $DefaultKeystore
        } elseif (Test-Path $LegacyKeystore) {
            $env:ELON_RELEASE_KEYSTORE = $LegacyKeystore
        }
    }
    if ([string]::IsNullOrWhiteSpace($env:ELON_RELEASE_KEY_ALIAS)) {
        $env:ELON_RELEASE_KEY_ALIAS = "elon"
    }
}

function Assert-ReleaseSigningConfig {
    Use-ReleaseSigningConfig

    $missing = @()
    if ([string]::IsNullOrWhiteSpace($env:ELON_RELEASE_KEYSTORE)) {
        $missing += "ELON_RELEASE_KEYSTORE（默认路径：$DefaultKeystore）"
    } elseif (-not (Test-Path $env:ELON_RELEASE_KEYSTORE)) {
        $missing += "ELON_RELEASE_KEYSTORE 文件不存在：$env:ELON_RELEASE_KEYSTORE"
    }
    if ([string]::IsNullOrWhiteSpace($env:ELON_RELEASE_STORE_PASSWORD)) {
        $missing += "ELON_RELEASE_STORE_PASSWORD"
    }
    if ([string]::IsNullOrWhiteSpace($env:ELON_RELEASE_KEY_ALIAS)) {
        $missing += "ELON_RELEASE_KEY_ALIAS"
    }
    if ([string]::IsNullOrWhiteSpace($env:ELON_RELEASE_KEY_PASSWORD)) {
        $missing += "ELON_RELEASE_KEY_PASSWORD"
    }

    if ($missing.Count -gt 0) {
        Write-Host ""
        Write-Host "缺少 APK 签名配置：" -ForegroundColor Yellow
        $missing | ForEach-Object { Write-Host "  - $_" -ForegroundColor Yellow }
        Write-Host ""
        Write-Host "一次性推荐设置：" -ForegroundColor Cyan
        Write-Host "  1. 将 elon-release.jks 放到 $DefaultKeystore" -ForegroundColor Cyan
        Write-Host "  2. 在用户环境变量或 ~/.gradle/gradle.properties 中配置：" -ForegroundColor Cyan
        Write-Host "     ELON_RELEASE_KEYSTORE=$DefaultKeystore" -ForegroundColor Cyan
        Write-Host "     ELON_RELEASE_STORE_PASSWORD=<不要提交到 git>" -ForegroundColor Cyan
        Write-Host "     ELON_RELEASE_KEY_ALIAS=elon" -ForegroundColor Cyan
        Write-Host "     ELON_RELEASE_KEY_PASSWORD=<不要提交到 git>" -ForegroundColor Cyan
        Write-Error "APK 签名配置不完整，已停止发布。"
    }
}

function Format-ShortSha {
    param([string]$Sha)

    if ([string]::IsNullOrWhiteSpace($Sha)) { return "" }
    if ($Sha.Length -le 7) { return $Sha }
    return $Sha.Substring(0, 7)
}

function Get-GitFetchFailureHint {
    param([string]$Output)

    $text = if ($Output) { $Output } else { "" }
    if ($text -match '(Could not resolve host|Name or service not known|Temporary failure in name resolution)') {
        return "网络/DNS 无法解析 GitHub，请检查网络、DNS 或代理后重试。"
    }
    if ($text -match '(Failed to connect|Connection timed out|Connection reset|Connection refused|Operation timed out|HTTP/2 stream|early EOF|The remote end hung up unexpectedly)') {
        return "网络连接到 GitHub 不稳定或超时，通常是临时抖动；脚本已短重试但仍失败。"
    }
    if ($text -match '(Permission denied|Authentication failed|Repository not found|Could not read from remote repository|Host key verification failed|publickey)') {
        return "Git 远端认证或仓库权限异常，请检查 SSH key、GitHub 权限和 origin 地址。"
    }
    return "Git fetch 失败，原因未能自动分类；请查看原始输出。"
}

function Invoke-GitFetchWithRetry {
    param(
        [string[]]$GitArgs = @("fetch", "origin", "main"),
        [int]$Attempts = 3,
        [int]$DelaySeconds = 2,
        [string]$FailureContext = "无法同步 origin/main",
        [switch]$Quiet
    )

    $lastOutput = ""
    for ($i = 1; $i -le $Attempts; $i++) {
        $oldPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $output = & git -C $RepoRoot -c http.proxy= -c https.proxy= @GitArgs 2>&1
        } finally {
            $ErrorActionPreference = $oldPreference
        }
        $lastOutput = ($output -join "`n").Trim()
        if ($LASTEXITCODE -eq 0) {
            if (-not $Quiet -and $i -gt 1) {
                Write-Host "   ✅ git fetch 重试成功（第 $i 次）" -ForegroundColor Green
            }
            return
        }

        if (-not $Quiet) {
            $hint = Get-GitFetchFailureHint -Output $lastOutput
            Write-Host "   ⚠️  git fetch 失败（第 $i/$Attempts 次）：$hint" -ForegroundColor Yellow
        }
        if ($i -lt $Attempts) { Start-Sleep -Seconds $DelaySeconds }
    }

    $finalHint = Get-GitFetchFailureHint -Output $lastOutput
    if (-not $Quiet) {
        Write-Host "CODE_SYNC_STATUS=unknown_fetch_failed"
        Write-Host "APK_RELEASE_STATUS=not_attempted"
        Write-Host "SERVER_RELEASE_STATUS=not_attempted"
    }
    Write-Error "$FailureContext：git $($GitArgs -join ' ') 连续失败 $Attempts 次。$finalHint 原始输出：$lastOutput"
}

function Write-ApkPublishStatus {
    param(
        [Parameter(Mandatory)] [string]$ApkReleaseStatus,
        [string]$CodeSyncStatus = "synced",
        [string]$ServerReleaseStatus = "not_attempted",
        [string]$Message = ""
    )

    if ($Message) {
        Write-Host "   $Message" -ForegroundColor Cyan
    }
    Write-Host "   CODE_SYNC_STATUS=$CodeSyncStatus" -ForegroundColor Gray
    Write-Host "   APK_RELEASE_STATUS=$ApkReleaseStatus" -ForegroundColor Gray
    Write-Host "   SERVER_RELEASE_STATUS=$ServerReleaseStatus" -ForegroundColor Gray
}

function Restore-GradleVersionFile {
    if ($null -eq $script:OriginalGradleContent) { return }
    [System.IO.File]::WriteAllText(
        $script:GradlePath,
        $script:OriginalGradleContent,
        (New-Object System.Text.UTF8Encoding($false))
    )
}

function Test-GitAncestor {
    param(
        [string]$Ancestor,
        [string]$Descendant
    )

    if ($Ancestor -notmatch '^[0-9a-f]{40}$' -or $Descendant -notmatch '^[0-9a-f]{40}$') {
        return $false
    }

    git -C $RepoRoot merge-base --is-ancestor $Ancestor $Descendant 2>$null | Out-Null
    return ($LASTEXITCODE -eq 0)
}

function Get-OriginMainSha {
    Invoke-GitFetchWithRetry -GitArgs @("fetch", "origin", "main", "--quiet") -FailureContext "无法判断 APK 构建是否已过期" -Quiet
    $sha = (git -C $RepoRoot rev-parse origin/main 2>$null).Trim()
    if ($LASTEXITCODE -ne 0 -or $sha -notmatch '^[0-9a-f]{40}$') {
        Write-Error "无法读取 origin/main SHA。"
    }
    return $sha
}

function Get-DeployedApkSha {
    # 优先 HTTP 查询 /app/version.json：快、不依赖 SSH、不会挂死。
    # SSH 仅作为 fallback，且必须设连接超时避免阻塞构建/轮询。
    try {
        $published = Invoke-HttpJson -Url "$ServerUrl/app/version.json" -TimeoutSec 10
        $sha = [string]$published.gitSha
        if ($sha -match '^[0-9a-f]{40}$') { return $sha }
    } catch {
        Write-Warning "无法从 /app/version.json 读取 APK gitSha：$_"
    }
    try {
        $raw = ssh -o ProxyCommand=none -o ConnectTimeout=10 -o ServerAliveInterval=5 -o ServerAliveCountMax=2 -o BatchMode=yes $ServerHost "cat $ApkShaFile 2>/dev/null || true" 2>$null
        $sha = ($raw | Out-String).Trim()
        if ($sha -match '^[0-9a-f]{40}$') { return $sha }
    } catch {
        Write-Warning "无法读取服务器 APK 部署 SHA：$_"
    }
    return $null
}

function Get-ApkBuildFreshness {
    param([string]$BaseSha)

    $remoteHead = Get-OriginMainSha
    $deployedSha = Get-DeployedApkSha

    if ($remoteHead -eq $BaseSha) {
        return [PSCustomObject]@{
            Action = "Continue"
            RemoteHead = $remoteHead
            DeployedSha = $deployedSha
            Reason = "origin/main 未变化"
        }
    }

    # Skip 仅当线上 APK 已包含本地全部待发布提交（含 ahead 的业务 commit）
    $skipCandidate = if ($script:LocalHeadSha) { $script:LocalHeadSha } else { $BaseSha }
    if ($deployedSha -and (Test-GitAncestor $skipCandidate $deployedSha)) {
        return [PSCustomObject]@{
            Action = "Skip"
            RemoteHead = $remoteHead
            DeployedSha = $deployedSha
            Reason = "服务器已部署包含本次基础提交的新 APK"
        }
    }

    return [PSCustomObject]@{
        Action = "Stale"
        RemoteHead = $remoteHead
        DeployedSha = $deployedSha
        Reason = "origin/main 在 APK 构建期间前进，但线上 APK 还未确认包含本次基础提交"
    }
}

function Stop-StaleApkRelease {
    param(
        [string]$Message,
        [switch]$Success
    )

    Restore-GradleVersionFile
    # 通知服务器释放本次预占的版本号槽位
    Complete-Release -Success:$false -ErrorMessage $Message
    if ($Success) {
        Write-Host "⏭️  $Message" -ForegroundColor Cyan
        Write-ApkPublishStatus -ApkReleaseStatus "superseded_by_newer_main" -Message "代码已合并，发布交给最新主线。"
        exit 0
    }
    Write-Error $Message
}

function Invoke-GradleReleaseBuild {
    Write-Host "🔨 编译 Release APK..." -ForegroundColor Cyan

    $gradle = Join-Path $AndroidDir "gradlew.bat"
    $buildStartedAt = Get-Date
    $process = Start-Process -FilePath "cmd.exe" `
        -ArgumentList @("/c", "`"$gradle`"", "assembleRelease") `
        -WorkingDirectory $AndroidDir `
        -NoNewWindow `
        -PassThru

    $lastPreemptCheck = [datetime]::MinValue
    while (-not $process.HasExited) {
        Start-Sleep -Seconds 10
        $process.Refresh()

        if (((Get-Date) - $lastPreemptCheck).TotalSeconds -lt 30) { continue }
        $lastPreemptCheck = Get-Date

        try {
            $freshness = Get-ApkBuildFreshness -BaseSha $BuildBaseSha
        } catch {
            Write-Warning "APK 编译中途并发检查失败，继续编译并在上传前再次检查：$_"
            continue
        }
        if ($freshness.Action -eq "Skip") {
            Write-Host "⏭️  检测到线上 APK 已更新到 $((Format-ShortSha $freshness.DeployedSha))，正在中止本地旧编译..." -ForegroundColor Cyan
            try {
                $process.Kill($true)
            } catch {
                taskkill /PID $process.Id /T /F | Out-Null
            }
            Stop-StaleApkRelease -Success -Message "线上 APK 已包含本次基础提交 $((Format-ShortSha $BuildBaseSha))，请直接测试服务器最新版本。"
        }
    }
    $process.WaitForExit()

    $exitCode = $process.ExitCode
    if ($null -eq $exitCode) {
        $candidateApk = Get-ChildItem $ApkPattern -ErrorAction SilentlyContinue |
            Sort-Object LastWriteTime |
            Select-Object -Last 1
        if ($candidateApk) {
            Write-Warning "Gradle 进程退出码为空，使用现有 release APK：$($candidateApk.Name)。后续 manifest 校验会确认版本是否匹配。"
            return
        }
        Write-Error "Gradle assembleRelease 结束但无法读取退出码，且未发现 release APK。"
    }

    if ($exitCode -ne 0) {
        Write-Error "Gradle assembleRelease 失败，退出码 $exitCode。"
    }
}

function Test-RemoteAdvanceSafeForApk {
    param([string]$BaseSha)
    # 检查 BaseSha..origin/main 区间是否只动了非 Android 文件；如是则 APK 不受影响，可安全 rebase。
    Invoke-GitFetchWithRetry -GitArgs @("fetch", "origin", "main") -FailureContext "无法判断远端前进是否影响 APK" -Quiet
    $changed = git -C $RepoRoot diff --name-only "$BaseSha..origin/main" 2>$null
    if (-not $changed) { return $true }
    foreach ($p in $changed) {
        if ($p -match '^android/' -or $p -match '^scripts/publish-apk') { return $false }
    }
    return $true
}

function Assert-ApkStillCurrentBeforeCommit {
    $freshness = Get-ApkBuildFreshness -BaseSha $BuildBaseSha
    if ($freshness.Action -eq "Continue") { return }

    if ($freshness.Action -eq "Skip") {
        Stop-StaleApkRelease -Success -Message "服务器已部署更新 APK（$((Format-ShortSha $freshness.DeployedSha))），且包含本次基础提交 $((Format-ShortSha $BuildBaseSha))。本次旧构建不再提交、不再上传。"
    }

    # 远端有新提交，但若全部都不影响 Android/发布脚本，则 APK 仍有效，允许继续发布。
    if (Test-RemoteAdvanceSafeForApk -BaseSha $BuildBaseSha) {
        Write-Host "   ℹ️  origin/main 前进到 $((Format-ShortSha $freshness.RemoteHead))，但新提交仅涉及非 Android 路径，本次 APK 仍可发布。" -ForegroundColor Cyan
        return
    }

    Stop-StaleApkRelease -Success -Message "origin/main 已从 $((Format-ShortSha $BuildBaseSha)) 前进到 $((Format-ShortSha $freshness.RemoteHead))，且包含 Android 改动，本次 APK 产物已过期。已还原 build.gradle；代码已合并，发布交给最新主线。"
}

function Assert-ApkStillCurrentBeforeUpload {
    param([string]$ReleaseSha)

    $remoteHead = Get-OriginMainSha
    $deployedSha = Get-DeployedApkSha

    if ($deployedSha -and (Test-GitAncestor $ReleaseSha $deployedSha)) {
        Write-Host "⏭️  服务器已部署包含本源代码提交的更新 APK：$((Format-ShortSha $deployedSha))" -ForegroundColor Cyan
        Complete-Release -Success:$false -ErrorMessage "superseded by deployed apk $deployedSha"
        Write-ApkPublishStatus -ApkReleaseStatus "published" -Message "APK 已由更新主线发布，当前代码已包含在线上 APK。"
        exit 0
    }

    if ($remoteHead -ne $ReleaseSha) {
        # 远端在编译期间前进；如果新增提交都不影响 Android，仍可安全发布
        if (Test-RemoteAdvanceSafeForApk -BaseSha $ReleaseSha) {
            Write-Host "   ℹ️  origin/main 已前进到 $((Format-ShortSha $remoteHead))，但新提交不影响 Android，继续发布。" -ForegroundColor Cyan
            return
        }
        Complete-Release -Success:$false -ErrorMessage "origin/main moved to $remoteHead and changed android files"
        Write-Host "⏭️  origin/main 已从本次基础 $((Format-ShortSha $ReleaseSha)) 前进到 $((Format-ShortSha $remoteHead))，且包含 Android 改动。为避免上传过期 APK，已停止；代码已合并，发布交给最新主线。" -ForegroundColor Cyan
        Write-ApkPublishStatus -ApkReleaseStatus "superseded_by_newer_main" -Message "代码已合并，发布交给最新主线。"
        exit 0
    }
}

function Publish-ApkStaged {
    param(
        [string]$ApkPath,
        [string]$JsonPath,
        [string]$ReleaseSha,
        [string]$ExpectedServerSha,
        [int]$Attempt = 1
    )

    $apkStage = "$ServerDir/ElonSpeed-latest.apk.$ReleaseSha.tmp"
    $jsonStage = "$ServerDir/version.json.$ReleaseSha.tmp"

    ssh -o ProxyCommand=none -o ConnectTimeout=15 -o ServerAliveInterval=10 -o ServerAliveCountMax=3 $ServerHost "mkdir -p $ServerDir"
    if ($LASTEXITCODE -ne 0) { Write-Error "无法创建服务器 APK 目录：$ServerDir" }

    scp -o ProxyCommand=none $ApkPath "${ServerHost}:${apkStage}"
    if ($LASTEXITCODE -ne 0) { Write-Error "APK staging 上传失败" }
    Write-Host "   ✅ APK staging 上传完成" -ForegroundColor Green

    scp -o ProxyCommand=none $JsonPath "${ServerHost}:${jsonStage}"
    if ($LASTEXITCODE -ne 0) { Write-Error "version.json staging 上传失败" }
    Write-Host "   ✅ version.json staging 上传完成" -ForegroundColor Green

    $remoteScript = @'
set -eu
APP_DIR='__APP_DIR__'
EXPECTED='__EXPECTED__'
NEW_SHA='__NEW_SHA__'
APK_STAGE='__APK_STAGE__'
JSON_STAGE='__JSON_STAGE__'
LOCK_FILE="$APP_DIR/.apk-deploy.lock"
SHA_FILE="$APP_DIR/.apk-deployed-sha"

(
  flock -x 9
  CURRENT=""
  if [ -f "$SHA_FILE" ]; then
    CURRENT="$(cat "$SHA_FILE" 2>/dev/null || true)"
  fi
  if [ "$CURRENT" != "$EXPECTED" ]; then
    echo "APK_DEPLOY_CAS_MISMATCH current=$CURRENT expected=$EXPECTED" >&2
    exit 42
  fi
  mv "$APK_STAGE" "$APP_DIR/ElonSpeed-latest.apk"
  mv "$JSON_STAGE" "$APP_DIR/version.json"
  printf '%s\n' "$NEW_SHA" > "$SHA_FILE"
) 9>"$LOCK_FILE"
'@

    $remoteScript = $remoteScript.
        Replace('__APP_DIR__', $ServerDir).
        Replace('__EXPECTED__', $ExpectedServerSha).
        Replace('__NEW_SHA__', $ReleaseSha).
        Replace('__APK_STAGE__', $apkStage).
        Replace('__JSON_STAGE__', $jsonStage)

    # PowerShell here-strings on Windows use CRLF; strip CR before piping to bash
    $remoteScript = $remoteScript -replace "`r`n", "`n"
    $remoteScript = $remoteScript -replace "`r", "`n"

    $remoteScript | ssh -o ProxyCommand=none -o ConnectTimeout=15 -o ServerAliveInterval=10 -o ServerAliveCountMax=3 $ServerHost "bash -s"
    $deployExit = $LASTEXITCODE

    if ($deployExit -eq 42) {
        $deployedSha = Get-DeployedApkSha
        if ($deployedSha -and (Test-GitAncestor $ReleaseSha $deployedSha)) {
            Write-Host "⏭️  另一台机器已部署更新 APK：$((Format-ShortSha $deployedSha))。本次 staging 不覆盖。" -ForegroundColor Cyan
            ssh -o ProxyCommand=none $ServerHost "rm -f '$apkStage' '$jsonStage'" | Out-Null
            Complete-Release -Success:$false -ErrorMessage "superseded by deployed apk $deployedSha"
            Write-ApkPublishStatus -ApkReleaseStatus "published" -Message "APK 已由更新主线发布，当前 staging 不覆盖。"
            exit 0
        }
        if ($deployedSha -and (Test-GitAncestor $deployedSha $ReleaseSha) -and $Attempt -lt 3) {
            Write-Host "   ℹ️  服务器刚部署了较旧 APK $((Format-ShortSha $deployedSha))，本 release $((Format-ShortSha $ReleaseSha)) 更新，重试原子发布..." -ForegroundColor Cyan
            Publish-ApkStaged -ApkPath $ApkPath -JsonPath $JsonPath -ReleaseSha $ReleaseSha -ExpectedServerSha $deployedSha -Attempt ($Attempt + 1)
            return
        }
        ssh -o ProxyCommand=none $ServerHost "rm -f '$apkStage' '$jsonStage'" | Out-Null
        Complete-Release -Success:$false -ErrorMessage "cas mismatch in apk deploy"
        Write-Host "⏭️  APK 上传 CAS 失败：服务器部署状态已变化，且未确认包含本源代码提交。本次 staging 不覆盖；代码已合并，发布交给最新主线。" -ForegroundColor Cyan
        Write-ApkPublishStatus -ApkReleaseStatus "superseded_by_newer_main" -Message "代码已合并，发布交给最新主线。"
        exit 0
    }

    if ($deployExit -ne 0) {
        Complete-Release -Success:$false -ErrorMessage "apk atomic deploy failed: exit=$deployExit"
        Write-Error "服务器 APK 原子发布失败，退出码 $deployExit"
    }
}

# ── Step 0: Git fetch + fast-forward（发布只基于远端最新 main） ─────────────

Write-Host "🔄 同步最新代码..." -ForegroundColor Cyan
Invoke-GitFetchWithRetry -GitArgs @("fetch", "origin", "main") -FailureContext "APK 发布未开始：无法同步 origin/main"

$dirty = (git -C $RepoRoot status --porcelain 2>$null) | Out-String
$dirty = $dirty.Trim()
if ($dirty) {
    Write-Host "❌ 工作区不干净，请先 commit + push 业务改动再运行 APK 发布脚本：" -ForegroundColor Red
    Write-Host $dirty -ForegroundColor Yellow
    Write-Error "工作区有未提交改动"
}

$localHeadBeforeSync = (git -C $RepoRoot rev-parse HEAD).Trim()
$originMainBeforeSync = (git -C $RepoRoot rev-parse origin/main).Trim()
if ($localHeadBeforeSync -ne $originMainBeforeSync) {
    git -C $RepoRoot merge-base --is-ancestor $localHeadBeforeSync $originMainBeforeSync | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "   ℹ️  本地 HEAD 已包含在 origin/main 中，快进到最新 main：$((Format-ShortSha $originMainBeforeSync))" -ForegroundColor Cyan
        git -C $RepoRoot merge --ff-only origin/main | Out-Null
        if ($LASTEXITCODE -ne 0) { Write-Error "git merge --ff-only origin/main 失败" }
    } else {
        git -C $RepoRoot merge-base --is-ancestor $originMainBeforeSync $localHeadBeforeSync | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Error "当前 HEAD 尚未进入 origin/main，禁止基于未推送提交发布 APK。请先执行：git push origin HEAD:main"
        }
        Write-Error "当前 HEAD 与 origin/main 已分叉，APK 发布脚本不会自动 rebase。请先完成代码合并并 push 后再运行。"
    }
}
$BuildBaseSha = (git -C $RepoRoot rev-parse HEAD).Trim()
$LocalHeadSha = $BuildBaseSha
$originMainSha = (git -C $RepoRoot rev-parse origin/main).Trim()
if ($BuildBaseSha -ne $originMainSha) {
    git -C $RepoRoot merge-base --is-ancestor $originMainSha $BuildBaseSha | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "   ℹ️  检测到本地存在待发布业务提交，APK freshness 基线改为 origin/main：$((Format-ShortSha $originMainSha))" -ForegroundColor Yellow
        $BuildBaseSha = $originMainSha
    }
}

# ── Step 1: 向服务器申请新的 versionName + versionCode（claim） ───────────

Write-Host "📝 向服务器申请新版本号..." -ForegroundColor Cyan

$content = Get-Content $GradlePath -Encoding UTF8 -Raw
$OriginalGradleContent = $content

$oldCode = [int]([regex]::Match($content, 'versionCode\s+(\d+)').Groups[1].Value)
$oldName = [regex]::Match($content, 'versionName\s+"([\d.]+)"').Groups[1].Value
Write-Host "   build.gradle 兜底: v$oldName (build $oldCode) — 不会被本次脚本提交" -ForegroundColor DarkGray

# 查服务器当前已部署版本，作为 claim 的唯一基准。
# build.gradle 只保留冷启动兜底，不允许在发布时当作版本基线。
$serverBaseline = Get-ServerApkVersionBaseline
$serverCurrentCode = [int]$serverBaseline.VersionCode
$serverCurrentName = [string]$serverBaseline.VersionName

if ($CurrentInstalledVersionCode -gt $serverCurrentCode) {
    $serverCurrentCode = $CurrentInstalledVersionCode
    if (-not [string]::IsNullOrWhiteSpace($CurrentInstalledVersionName)) {
        $serverCurrentName = $CurrentInstalledVersionName
    }
    Write-Host "   ℹ️  已安装版本基线: build $serverCurrentCode (v$serverCurrentName)，以此为 claim 最低基准" -ForegroundColor DarkGray
}

$builderId = "$env:COMPUTERNAME-$env:USERNAME"
if ([string]::IsNullOrWhiteSpace($builderId) -or $builderId -eq "-") {
    $builderId = "unknown-builder-" + ([Guid]::NewGuid().ToString().Substring(0,8))
}
$builderLabel = "publish-apk.ps1 @ $builderId"

try {
    $claim = Invoke-ReleaseApi -Endpoint 'claim' -Body (@{
        kind               = 'apk'
        sha                = $BuildBaseSha
        builderId          = $builderId
        builderLabel       = $builderLabel
        bump               = 'patch'
        currentVersionName = $serverCurrentName
        currentVersionCode = $serverCurrentCode
    })
} catch {
    Write-Error "❌ /api/release/claim 失败：$_"
}

if (-not $claim -or $claim.action -ne 'build') {
    Write-Error "❌ release/claim 返回非预期响应：$($claim | ConvertTo-Json -Compress)"
}

$script:ReleaseToken = [string]$claim.token
$newName = [string]$claim.assignedVersionName
$newCode = [int]$claim.assignedVersionCode
if ([string]::IsNullOrWhiteSpace($newName) -or $newCode -le 0) {
    Write-Error "❌ release/claim 未返回有效的 assignedVersionName/Code: $($claim | ConvertTo-Json -Compress)"
}
if ($newCode -le $serverCurrentCode) {
    Complete-Release -Success:$false -ErrorMessage "claim returned non-incrementing apk build $newCode from baseline $serverCurrentCode"
    Write-Error "❌ release/claim 分配的 build $newCode 未高于基线 build $serverCurrentCode，已停止发布。"
}
$InFlightCount = if ($claim.PSObject.Properties.Match('inFlightCount').Count) { [int]$claim.inFlightCount } else { 1 }
Write-Host "   ✅ 已分配版本号: v$newName (build $newCode) [token=$($script:ReleaseToken.Substring(0,8))..., in-flight=$InFlightCount]" -ForegroundColor Green

# 把分配到的版本号临时写入 build.gradle（编译时 AGP 必须能读到），编译完成后通过 Restore-GradleVersionFile 还原。
$content = $content -replace "versionCode\s+$oldCode", "versionCode $newCode"
$escapedOldName = [regex]::Escape($oldName)
$content = $content -replace "versionName\s+`"$escapedOldName`"", "versionName `"$newName`""
[System.IO.File]::WriteAllText(
    $GradlePath,
    $content,
    (New-Object System.Text.UTF8Encoding($false))
)

$versionName = $newName

Write-Host "   versionCode: $oldCode → $newCode (临时写入 build.gradle，编译后自动还原)" -ForegroundColor Green
Write-Host "   versionName: $oldName → $newName (临时写入 build.gradle，编译后自动还原)" -ForegroundColor Green

# ── Step 2: 编译 APK ─────────────────────────────────────────────────────────

if (-not $SkipBuild) {
    Assert-ReleaseSigningConfig
    Invoke-GradleReleaseBuild
} else {
    Write-Host "⏭️  跳过编译（-SkipBuild）" -ForegroundColor Yellow
}

Assert-ApkStillCurrentBeforeCommit

# ── Step 3: 找到 APK 文件 ─────────────────────────────────────────────────────

$apk = Get-ChildItem $ApkPattern -ErrorAction SilentlyContinue | Select-Object -Last 1
if (-not $apk) {
    Write-Error "❌ 未找到 APK 文件，路径: $ApkPattern"
    exit 1
}

$fileSize = $apk.Length
Write-Host "📦 APK: $($apk.Name) ($([math]::Round($fileSize / 1MB, 2)) MB)" -ForegroundColor Green
Assert-ApkManifestVersion -ApkPath $apk.FullName -ExpectedVersionCode $newCode -ExpectedVersionName $versionName -Label "本地 release APK"

# ── Step 4: 还原 build.gradle（版本号不进 git） ──────────────────────────────

Write-Host "🧹 还原 build.gradle 到 git 兜底版本（v$oldName / build $oldCode）..." -ForegroundColor Cyan
Restore-GradleVersionFile

# 本次发布的 git SHA 直接采用基础源代码提交（不会新增版本号提交）。
# version.json 的 gitSha = 本次实际编译用的源代码 SHA。
$shaFull = $BuildBaseSha
$sha = $shaFull.Substring(0,7)
Write-Host "   本次发布对应源 SHA: $sha (无新增版本号提交)" -ForegroundColor Green

Assert-ApkStillCurrentBeforeUpload -ReleaseSha $shaFull
$serverShaBeforeUpload = Get-DeployedApkSha
if ($null -eq $serverShaBeforeUpload) { $serverShaBeforeUpload = "" }

# ── Step 5: 生成 version.json ─────────────────────────────────────────────────

$downloadUrl = "$ServerUrl/app/ElonSpeed-latest.apk"
$versionJson = @{
    versionCode = $newCode
    versionName = $versionName
    downloadUrl = $downloadUrl
    changelog   = $Changelog
    forceUpdate = $false
    fileSize    = $fileSize
    gitSha      = $shaFull
    sourceSha   = $BuildBaseSha
} | ConvertTo-Json -Depth 2

$tmpJson = Join-Path $env:TEMP "elon-version.json"
$versionJson | Set-Content $tmpJson -Encoding UTF8
Write-Host "📋 version.json 已生成" -ForegroundColor Green

# ── Step 5.5: 上传前再次检查服务器是否已发布更新版本 ─────────────────────
# 场景：本机编译耗时较长，期间另一台 PC 可能已经发布了更高 versionCode 的 APK。
# 本机上传会覆盖服务器上更新的 version.json 与 APK，导致手机端看到版本倍退。
# 与 publish-server.ps1 里的祖先检查策略类似：服务器已有更新版本 → 中止上传。
if (-not $Force) {
    try {
        $serverNow = Invoke-HttpJson -Url "$ServerUrl/app/version.json" -TimeoutSec 10
        $serverNowCode = [int]$serverNow.versionCode
        # 服务器 SHA（version.json 里可能是 sourceSha 或 gitSha）
        $serverNowSha = if ($serverNow.sourceSha) { [string]$serverNow.sourceSha } else { [string]$serverNow.gitSha }
        $serverNowShaShort = if ($serverNowSha.Length -ge 7) { $serverNowSha.Substring(0,7) } else { $serverNowSha }

        # 判断逻辑：
        #   serverNowCode > newCode          → 真的有更新版本，中止
        #   serverNowCode == newCode
        #     且 server SHA == 本机 SHA      → 完全相同，已发布，中止
        #     且 server SHA != 本机 SHA      → 旧 SHA 占用了同一版本号槽，本机代码更新，继续覆盖
        $serverIsAhead   = $serverNowCode -gt $newCode
        $sameCodeSameSha = ($serverNowCode -eq $newCode) -and ($serverNowShaShort -eq $sha)

        if ($serverIsAhead -or $sameCodeSameSha) {
            Write-Host ""
            Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
            Write-Host "   ⚠️  APK 发布已中止：服务器已有更新版本" -ForegroundColor Yellow
            Write-Host "   服务器当前：build $serverNowCode (v$($serverNow.versionName)) · SHA $serverNowShaShort" -ForegroundColor Yellow
            Write-Host "   本次编译：  build $newCode (v$versionName) · SHA $sha" -ForegroundColor Yellow
            if ($serverIsAhead) {
                Write-Host "   原因：另一台 PC 在本机编译期间已经发布了更高版本号的 APK。" -ForegroundColor Yellow
            } else {
                Write-Host "   原因：服务器已有完全相同的版本（同 build 号 + 同 SHA），无需重复发布。" -ForegroundColor Yellow
            }
            Write-Host "   处理：代码已合并，发布交给最新主线；本次本机编译的 APK 作废，服务器分配的 build $newCode 槽位将释放回 in-flight 列表。" -ForegroundColor Yellow
            Write-Host "   如需本机验证已发布的新版 APK，直接下载 $ServerUrl/app/ElonSpeed-latest.apk。" -ForegroundColor Yellow
            Write-Host "   如确要覆盖（不推荐）：重跑加 -Force。" -ForegroundColor Yellow
            Write-ApkPublishStatus -ApkReleaseStatus "superseded_by_newer_main" -Message "代码已合并，发布交给最新主线。"
            Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
            Write-Host ""
            Complete-Release -Success:$false -ErrorMessage "server already has newer apk: build $serverNowCode"
            exit 0
        }

        if ($serverNowCode -eq $newCode) {
            # serverNowCode == newCode 且 SHA 不同 → 旧代码占槽，本次代码更新，覆盖
            Write-Host "   ℹ️  服务器有同版本号 build $serverNowCode 但 SHA 不同（服务器:$serverNowShaShort 本机:$sha），旧槽被旧代码占用，继续覆盖..." -ForegroundColor Cyan
        } else {
            Write-Host "   ✅ 服务器版本检查通过（服务器 $serverNowCode < 本次 $newCode）" -ForegroundColor Green
        }
    } catch {
        Complete-Release -Success:$false -ErrorMessage "could not read server apk version before upload"
        Write-Error "❌ 上传前无法读取服务器 version.json，已停止发布，避免覆盖服务器上的未知新版本：$_"
    }
}
# ── Step 6: SCP 上传到服务器 ──────────────────────────────────────────────────

Write-Host "🚀 上传到服务器..." -ForegroundColor Cyan

Publish-ApkStaged -ApkPath $apk.FullName -JsonPath $tmpJson -ReleaseSha $shaFull -ExpectedServerSha $serverShaBeforeUpload
Write-Host "   ✅ APK 原子发布完成，.apk-deployed-sha = $sha" -ForegroundColor Green

# 清理临时文件
Remove-Item $tmpJson -Force

# ── Step 7: 验证 ──────────────────────────────────────────────────────────────

Write-Host "🔍 验证服务器响应..." -ForegroundColor Cyan
Start-Sleep -Seconds 1

try {
    $resp = Invoke-HttpJson -Url "$ServerUrl/app/version.json" -TimeoutSec 10
    Write-Host "   服务器返回: v$($resp.versionName) (build $($resp.versionCode))" -ForegroundColor Green
    if ($resp.versionCode -eq $newCode) {
        Write-Host "   ✅ versionCode 一致，发布成功！" -ForegroundColor Green
    } else {
        Write-Warning "   ⚠️  服务器 versionCode=$($resp.versionCode)，期望 $newCode"
    }
} catch {
    Write-Warning "   ⚠️  验证请求失败: $_（可能服务端重启中，稍后手动验证）"
}
Assert-RemoteApkManifestVersion -ExpectedVersionCode $newCode -ExpectedVersionName $versionName

Write-Host "📣 广播在线客户端更新提醒..." -ForegroundColor Cyan
try {
    $broadcastUrl = "$ServerUrl/api/app/update/broadcast"
    $headers = @{}
    if (-not [string]::IsNullOrWhiteSpace($env:APP_UPDATE_BROADCAST_TOKEN)) {
        $headers["Authorization"] = "Bearer $env:APP_UPDATE_BROADCAST_TOKEN"
    }
    $broadcastParams = @{
        Method = "Post"
        Uri = $broadcastUrl
        Headers = $headers
        TimeoutSec = 10
    }
    $broadcastParams = Add-ElonProjectDirectRequestParameters -Params $broadcastParams -CommandName "Invoke-RestMethod"
    $broadcast = Invoke-RestMethod @broadcastParams
    Write-Host "   ✅ 已通知在线连接: $($broadcast.receivers)" -ForegroundColor Green
} catch {
    Write-Warning "   ⚠️  实时广播失败: $_（不影响 APK 发布，离线/未收到用户仍会定期检查）"
}

# ── 汇报 ──────────────────────────────────────────────────────────────────────

Complete-Release -Success:$true -VersionName $versionName -VersionCode $newCode -Sha $shaFull

Write-Host ""
Write-Host ("=" * 60) -ForegroundColor Cyan
Write-Host "✅ 发布完成！" -ForegroundColor Green
Write-Host "   版本: v$versionName (build $newCode) — 服务器分配，未写入 git" -ForegroundColor White
Write-Host "   SHA:  $sha (源代码提交，无新增版本号提交)" -ForegroundColor White
Write-Host "   下载: $downloadUrl" -ForegroundColor White
Write-ApkPublishStatus -ApkReleaseStatus "published"
Write-Host ("=" * 60) -ForegroundColor Cyan

# ── 启动局域网分发种子服务（后台无窗口，多项目共享守护进程）─────────────────

Write-Host ""
Write-Host "📡 注册局域网分发服务（同WiFi直接下载，无需走服务器）..." -ForegroundColor Cyan
$distClient = Join-Path $PSScriptRoot "lan-dist-client.ps1"
if (Test-Path $distClient) {
    try {
        # 客户端模式：写注册文件并确保守护进程在后台运行（立即返回）
        & $distClient `
            -ProjectId         "elon" `
            -ArtifactId        "user-apk" `
            -FilePath          $apk.FullName `
            -VersionCode       $newCode `
            -ServerRegisterUrl "$ServerUrl/app/lan-peer/register"
    } catch {
        Write-Warning "   ⚠️  LAN 分发服务启动失败: $_（不影响 APK 发布，手机仍可从服务器下载）"
    }
} else {
    Write-Warning "   ⚠️  未找到 $distClient，跳过局域网分发服务"
}

# ── 自动清理已合并、工作树干净的孤儿 task worktree ─────────────
$cleanupScript = Join-Path $RepoRoot "scripts\cleanup-task-worktrees.ps1"
if (Test-Path -LiteralPath $cleanupScript) {
    try {
        $cleanupOut = & powershell -NoProfile -ExecutionPolicy Bypass -File $cleanupScript -Apply 2>&1
        $removedLine = $cleanupOut | Select-String -Pattern "^完成：清理" | Select-Object -Last 1
        if ($removedLine) {
            Write-Host "   $($removedLine.Line.Trim())（自动）" -ForegroundColor DarkGray
        }
    } catch {
        Write-Host "   ⚠️  自动清理 worktree 失败：$_" -ForegroundColor Yellow
    }
}
