<#
.SYNOPSIS
    一龙 Android APK 发布脚本（版本号由服务器分配，build.gradle 不再进 git）

.DESCRIPTION
    新版业务流程（version-from-server）：
      1. git pull --rebase origin main      (业务 commit 必须先由 AI 自己 push)
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
    [switch]$Force
)

$ErrorActionPreference = "Stop"

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
        if ($script:ReleaseToken -and -not $script:ReleaseFinished) {
            Complete-Release -Success:$false -ErrorMessage ("uncaught error: " + ($_ | Out-String))
        }
    } catch {}
    # 让原始错误继续抛出（脚本仍按 $ErrorActionPreference=Stop 终止）
    continue
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
    git -C $RepoRoot fetch origin main --quiet
    if ($LASTEXITCODE -ne 0) { Write-Error "git fetch origin main 失败，无法判断 APK 构建是否已过期。" }
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
        $published = Invoke-RestMethod "$ServerUrl/app/version.json" -TimeoutSec 10 -NoProxy
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
        $freshApk = Get-ChildItem $ApkPattern -ErrorAction SilentlyContinue |
            Where-Object { $_.LastWriteTime -ge $buildStartedAt } |
            Sort-Object LastWriteTime |
            Select-Object -Last 1
        if ($freshApk) {
            Write-Warning "Gradle 进程退出码为空，但本次构建已产出 release APK：$($freshApk.Name)。继续发布。"
            return
        }
        Write-Error "Gradle assembleRelease 结束但无法读取退出码，且未发现本次新 APK。"
    }

    if ($exitCode -ne 0) {
        Write-Error "Gradle assembleRelease 失败，退出码 $exitCode。"
    }
}

function Test-RemoteAdvanceSafeForApk {
    param([string]$BaseSha)
    # 检查 BaseSha..origin/main 区间是否只动了非 Android 文件；如是则 APK 不受影响，可安全 rebase。
    git -C $RepoRoot fetch origin main 2>$null | Out-Null
    $changed = git -C $RepoRoot diff --name-only "$BaseSha..origin/main" 2>$null
    if (-not $changed) { return $true }
    foreach ($p in $changed) {
        if ($p -match '^android/' -or $p -match '^scripts/publish-apk') { return $false }
    }
    return $true
}

function Push-HeadToMain {
    for ($i = 1; $i -le 4; $i++) {
        git -C $RepoRoot push origin HEAD:main
        if ($LASTEXITCODE -eq 0) { return }
        Write-Host "   ⚠️  git push 被拒绝（第 $i 次），尝试 rebase 最新 origin/main 后重推..." -ForegroundColor Yellow
        git -C $RepoRoot fetch origin main | Out-Null
        $base = git -C $RepoRoot merge-base HEAD origin/main
        if (-not (Test-RemoteAdvanceSafeForApk -BaseSha $base)) {
            Write-Error "远端 origin/main 自基线后包含 Android 改动，rebase 不安全，请人工同步后重发。"
            return
        }
        git -C $RepoRoot rebase origin/main
        if ($LASTEXITCODE -ne 0) {
            Write-Error "自动 rebase 失败，请人工解决冲突后重发。"
            return
        }
        Start-Sleep -Seconds 2
    }
    Write-Error "连续 4 次推送均被拒绝，放弃自动重试。"
}

function Assert-ApkStillCurrentBeforeCommit {
    $freshness = Get-ApkBuildFreshness -BaseSha $BuildBaseSha
    if ($freshness.Action -eq "Continue") { return }

    if ($freshness.Action -eq "Skip") {
        Stop-StaleApkRelease -Success -Message "服务器已部署更新 APK（$((Format-ShortSha $freshness.DeployedSha))），且包含本次基础提交 $((Format-ShortSha $BuildBaseSha))。本次旧构建不再提交、不再上传。"
    }

    # 远端有新提交，但若全部都不影响 Android/发布脚本，则 APK 仍有效，允许 commit 后自动 rebase 重推
    if (Test-RemoteAdvanceSafeForApk -BaseSha $BuildBaseSha) {
        Write-Host "   ℹ️  origin/main 前进到 $((Format-ShortSha $freshness.RemoteHead))，但新提交仅涉及非 Android 路径，本次 APK 仍可发布（commit 后自动 rebase）。" -ForegroundColor Cyan
        return
    }

    Stop-StaleApkRelease -Message "origin/main 已从 $((Format-ShortSha $BuildBaseSha)) 前进到 $((Format-ShortSha $freshness.RemoteHead))，且包含 Android 改动，本次 APK 产物已过期。已还原 build.gradle，请基于最新 main 重新运行发布脚本。"
}

function Assert-ApkStillCurrentBeforeUpload {
    param([string]$ReleaseSha)

    $remoteHead = Get-OriginMainSha
    $deployedSha = Get-DeployedApkSha

    if ($deployedSha -and (Test-GitAncestor $ReleaseSha $deployedSha)) {
        Write-Host "⏭️  服务器已部署包含本 release commit 的更新 APK：$((Format-ShortSha $deployedSha))" -ForegroundColor Cyan
        Complete-Release -Success:$false -ErrorMessage "superseded by deployed apk $deployedSha"
        exit 0
    }

    if ($remoteHead -ne $ReleaseSha) {
        # 远端在编译期间前进；如果新增提交都不影响 Android，仍可安全发布
        if (Test-RemoteAdvanceSafeForApk -BaseSha $ReleaseSha) {
            Write-Host "   ℹ️  origin/main 已前进到 $((Format-ShortSha $remoteHead))，但新提交不影响 Android，继续发布。" -ForegroundColor Cyan
            return
        }
        Complete-Release -Success:$false -ErrorMessage "origin/main moved to $remoteHead and changed android files"
        Write-Error "origin/main 已从本次基础 $((Format-ShortSha $ReleaseSha)) 前进到 $((Format-ShortSha $remoteHead))，且包含 Android 改动。为避免上传过期 APK，已停止；请重新运行发布脚本。"
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
            exit 0
        }
        if ($deployedSha -and (Test-GitAncestor $deployedSha $ReleaseSha) -and $Attempt -lt 3) {
            Write-Host "   ℹ️  服务器刚部署了较旧 APK $((Format-ShortSha $deployedSha))，本 release $((Format-ShortSha $ReleaseSha)) 更新，重试原子发布..." -ForegroundColor Cyan
            Publish-ApkStaged -ApkPath $ApkPath -JsonPath $JsonPath -ReleaseSha $ReleaseSha -ExpectedServerSha $deployedSha -Attempt ($Attempt + 1)
            return
        }
        ssh -o ProxyCommand=none $ServerHost "rm -f '$apkStage' '$jsonStage'" | Out-Null
        Complete-Release -Success:$false -ErrorMessage "cas mismatch in apk deploy"
        Write-Error "APK 上传 CAS 失败：服务器部署状态已变化，但未确认包含本 release commit。请基于最新 main 重新发布。"
    }

    if ($deployExit -ne 0) {
        Complete-Release -Success:$false -ErrorMessage "apk atomic deploy failed: exit=$deployExit"
        Write-Error "服务器 APK 原子发布失败，退出码 $deployExit"
    }
}

# ── Step 0: Git pull（防止 push 冲突） ────────────────────────────────────────

Write-Host "🔄 同步最新代码..." -ForegroundColor Cyan
git -C $RepoRoot pull --rebase origin main
if ($LASTEXITCODE -ne 0) { Write-Error "git pull --rebase 失败" }
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
        currentVersionName = $oldName
        currentVersionCode = $oldCode
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

# ── Step 4: 还原 build.gradle（版本号不进 git） ──────────────────────────────

Write-Host "🧹 还原 build.gradle 到 git 兜底版本（v$oldName / build $oldCode）..." -ForegroundColor Cyan
Restore-GradleVersionFile

# 本次发布的 git SHA 直接采用基础 commit（没有新的 release commit）。
# version.json 的 gitSha = 本次实际编译用的源代码 SHA。
$shaFull = $BuildBaseSha
$sha = $shaFull.Substring(0,7)
Write-Host "   本次发布对应源 SHA: $sha (无新增 release commit)" -ForegroundColor Green

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
        $serverNow = Invoke-RestMethod "$ServerUrl/app/version.json" -TimeoutSec 10 -NoProxy
        $serverNowCode = [int]$serverNow.versionCode
        if ($serverNowCode -ge $newCode) {
            Write-Host ""
            Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
            Write-Host "   ⚠️  APK 发布已中止：服务器已有更新版本" -ForegroundColor Yellow
            Write-Host "   服务器当前：build $serverNowCode (v$($serverNow.versionName))" -ForegroundColor Yellow
            Write-Host "   本次编译：  build $newCode (v$versionName)" -ForegroundColor Yellow
            Write-Host "   原因：另一台 PC 在本机编译期间已经发布了同等或更新的 APK。" -ForegroundColor Yellow
            Write-Host "   处理：本次本机编译的 APK 作废；服务器分配的 build $newCode 槽位将释放回 in-flight 列表。" -ForegroundColor Yellow
            Write-Host "   如需本机验证已发布的新版 APK，直接下载 $ServerUrl/app/ElonSpeed-latest.apk。" -ForegroundColor Yellow
            Write-Host "   如确要覆盖（不推荐）：重跑加 -Force。" -ForegroundColor Yellow
            Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
            Write-Host ""
            Complete-Release -Success:$false -ErrorMessage "server already has newer apk: build $serverNowCode"
            exit 0
        }
        Write-Host "   ✅ 服务器版本检查通过（服务器 $serverNowCode < 本次 $newCode）" -ForegroundColor Green
    } catch {
        Write-Warning "   ⚠️  上传前无法读取服务器 version.json，跳过祖先检查：$_"
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
    $resp = Invoke-RestMethod "$ServerUrl/app/version.json" -TimeoutSec 10
    Write-Host "   服务器返回: v$($resp.versionName) (build $($resp.versionCode))" -ForegroundColor Green
    if ($resp.versionCode -eq $newCode) {
        Write-Host "   ✅ versionCode 一致，发布成功！" -ForegroundColor Green
    } else {
        Write-Warning "   ⚠️  服务器 versionCode=$($resp.versionCode)，期望 $newCode"
    }
} catch {
    Write-Warning "   ⚠️  验证请求失败: $_（可能服务端重启中，稍后手动验证）"
}

Write-Host "📣 广播在线客户端更新提醒..." -ForegroundColor Cyan
try {
    $broadcastUrl = "$ServerUrl/api/app/update/broadcast"
    $headers = @{}
    if (-not [string]::IsNullOrWhiteSpace($env:APP_UPDATE_BROADCAST_TOKEN)) {
        $headers["Authorization"] = "Bearer $env:APP_UPDATE_BROADCAST_TOKEN"
    }
    $broadcast = Invoke-RestMethod -Method Post -Uri $broadcastUrl -Headers $headers -TimeoutSec 10
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
Write-Host "   SHA:  $sha (源代码 commit，无新增 release commit)" -ForegroundColor White
Write-Host "   下载: $downloadUrl" -ForegroundColor White
Write-Host ("=" * 60) -ForegroundColor Cyan

# ── 启动局域网 APK 种子服务（后台无窗口）────────────────────────────────────

Write-Host ""
Write-Host "📡 启动局域网 APK 种子服务（同WiFi直接下载，无需走服务器）..." -ForegroundColor Cyan
$lanScript = Join-Path $PSScriptRoot "lan-apk-server.ps1"
if (Test-Path $lanScript) {
    try {
        # -WindowStyle Hidden：不出现控制台窗口
        # -NonInteractive：不等待用户输入
        Start-Process pwsh -WindowStyle Hidden -ArgumentList @(
            "-NonInteractive",
            "-ExecutionPolicy", "Bypass",
            "-File", $lanScript,
            "-ApkPath", $apk.FullName,
            "-VersionCode", $newCode,
            "-ServerUrl", $ServerUrl
        )
        Write-Host "   ✅ LAN 种子服务已在后台启动（端口 7788，2 小时后自动退出）" -ForegroundColor Green
        Write-Host "   📄 日志: $env:TEMP\elon-lan-apk-server.log" -ForegroundColor DarkGray
    } catch {
        Write-Warning "   ⚠️  LAN 种子服务启动失败: $_（不影响 APK 发布，手机仍可从服务器下载）"
    }
} else {
    Write-Warning "   ⚠️  未找到 $lanScript，跳过局域网种子服务"
}
