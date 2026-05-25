<#
.SYNOPSIS
    一龙 Android APK 发布脚本

.DESCRIPTION
    自动完成：versionCode +1 → 编译 APK → 生成 version.json → SCP 上传服务器 → 验证

.PARAMETER Changelog
    本次版本更新说明（必填）

.PARAMETER SkipBuild
    跳过 Gradle 编译，直接用已有的 APK 重新上传（用于调试脚本）

.EXAMPLE
    .\publish-apk.ps1 -Changelog "修复启动闪退"
    .\publish-apk.ps1 -SkipBuild -Changelog "仅重新上传"
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$Changelog,

    [switch]$SkipBuild,

    # 跳过上传前的“服务器已发布更新 versionCode”检查，强制覆盖
    [switch]$Force
)

$ErrorActionPreference = "Stop"

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
    try {
        $raw = ssh -o ProxyCommand=none $ServerHost "cat $ApkShaFile 2>/dev/null || true" 2>$null
        $sha = ($raw | Out-String).Trim()
        if ($sha -match '^[0-9a-f]{40}$') { return $sha }
    } catch {
        Write-Warning "无法读取服务器 APK 部署 SHA：$_"
    }
    try {
        $published = Invoke-RestMethod "$ServerUrl/app/version.json" -TimeoutSec 10
        $sha = [string]$published.gitSha
        if ($sha -match '^[0-9a-f]{40}$') { return $sha }
    } catch {
        Write-Warning "无法从 /app/version.json 读取 APK gitSha：$_"
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

    if ($deployedSha -and (Test-GitAncestor $BaseSha $deployedSha)) {
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

function Push-HeadToMain {
    git -C $RepoRoot push origin HEAD:main
    if ($LASTEXITCODE -eq 0) { return }

    Write-Error @"
git push 被拒绝。APK 已经按旧 HEAD 编译，脚本不会 rebase 后继续上传旧产物。
请同步最新 main 后重新运行：
  git fetch origin main
  git rebase origin/main
  scripts\publish-apk.ps1 -Changelog "$Changelog"
"@
}

function Assert-ApkStillCurrentBeforeCommit {
    $freshness = Get-ApkBuildFreshness -BaseSha $BuildBaseSha
    if ($freshness.Action -eq "Continue") { return }

    if ($freshness.Action -eq "Skip") {
        Stop-StaleApkRelease -Success -Message "服务器已部署更新 APK（$((Format-ShortSha $freshness.DeployedSha))），且包含本次基础提交 $((Format-ShortSha $BuildBaseSha))。本次旧构建不再提交、不再上传。"
    }

    Stop-StaleApkRelease -Message "origin/main 已从 $((Format-ShortSha $BuildBaseSha)) 前进到 $((Format-ShortSha $freshness.RemoteHead))，本次 APK 产物已过期。已还原 build.gradle，请基于最新 main 重新运行发布脚本。"
}

function Assert-ApkStillCurrentBeforeUpload {
    param([string]$ReleaseSha)

    $remoteHead = Get-OriginMainSha
    $deployedSha = Get-DeployedApkSha

    if ($deployedSha -and (Test-GitAncestor $ReleaseSha $deployedSha)) {
        Write-Host "⏭️  服务器已部署包含本 release commit 的更新 APK：$((Format-ShortSha $deployedSha))" -ForegroundColor Cyan
        exit 0
    }

    if ($remoteHead -ne $ReleaseSha) {
        Write-Error "origin/main 已从本 release commit $((Format-ShortSha $ReleaseSha)) 前进到 $((Format-ShortSha $remoteHead))。为避免上传旧 APK，已停止；请基于最新 main 重新运行发布脚本。"
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

    ssh -o ProxyCommand=none $ServerHost "mkdir -p $ServerDir"
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

    $remoteScript | ssh -o ProxyCommand=none $ServerHost "bash -s"
    $deployExit = $LASTEXITCODE

    if ($deployExit -eq 42) {
        $deployedSha = Get-DeployedApkSha
        if ($deployedSha -and (Test-GitAncestor $ReleaseSha $deployedSha)) {
            Write-Host "⏭️  另一台机器已部署更新 APK：$((Format-ShortSha $deployedSha))。本次 staging 不覆盖。" -ForegroundColor Cyan
            ssh -o ProxyCommand=none $ServerHost "rm -f '$apkStage' '$jsonStage'" | Out-Null
            exit 0
        }
        if ($deployedSha -and (Test-GitAncestor $deployedSha $ReleaseSha) -and $Attempt -lt 3) {
            Write-Host "   ℹ️  服务器刚部署了较旧 APK $((Format-ShortSha $deployedSha))，本 release $((Format-ShortSha $ReleaseSha)) 更新，重试原子发布..." -ForegroundColor Cyan
            Publish-ApkStaged -ApkPath $ApkPath -JsonPath $JsonPath -ReleaseSha $ReleaseSha -ExpectedServerSha $deployedSha -Attempt ($Attempt + 1)
            return
        }
        ssh -o ProxyCommand=none $ServerHost "rm -f '$apkStage' '$jsonStage'" | Out-Null
        Write-Error "APK 上传 CAS 失败：服务器部署状态已变化，但未确认包含本 release commit。请基于最新 main 重新发布。"
    }

    if ($deployExit -ne 0) {
        Write-Error "服务器 APK 原子发布失败，退出码 $deployExit"
    }
}

# ── Step 0: Git pull（防止 push 冲突） ────────────────────────────────────────

Write-Host "🔄 同步最新代码..." -ForegroundColor Cyan
git -C $RepoRoot pull --rebase origin main
if ($LASTEXITCODE -ne 0) { Write-Error "git pull --rebase 失败" }
$BuildBaseSha = (git -C $RepoRoot rev-parse HEAD).Trim()
$originMainSha = (git -C $RepoRoot rev-parse origin/main).Trim()
if ($BuildBaseSha -ne $originMainSha) {
    git -C $RepoRoot merge-base --is-ancestor $originMainSha $BuildBaseSha | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "   ℹ️  检测到本地存在待发布业务提交，APK freshness 基线改为 origin/main：$((Format-ShortSha $originMainSha))" -ForegroundColor Yellow
        $BuildBaseSha = $originMainSha
    }
}

# ── Step 1: 递增 versionCode，确认 versionName ────────────────────────────────

Write-Host "📝 更新版本号..." -ForegroundColor Cyan

$content = Get-Content $GradlePath -Encoding UTF8 -Raw
$OriginalGradleContent = $content

$oldCode = [int]([regex]::Match($content, 'versionCode\s+(\d+)').Groups[1].Value)
$publishedCode = 0
$publishedName = $null
try {
    $published = Invoke-RestMethod "$ServerUrl/app/version.json" -TimeoutSec 10
    $publishedCode = [int]$published.versionCode
    $publishedName = [string]$published.versionName
    Write-Host "   线上版本: v$publishedName (build $publishedCode)" -ForegroundColor Gray
} catch {
    Write-Warning "   ⚠️  无法读取线上版本，将仅基于本地 build.gradle 递增：$_"
}
$baseCode = [Math]::Max($oldCode, $publishedCode)
$newCode = $baseCode + 1
$content = $content -replace "versionCode\s+$oldCode", "versionCode $newCode"

# 自动递增 versionName PATCH（1.0 → 1.0.1，1.0.1 → 1.0.2，1.2 → 1.2.1）
$oldName = [regex]::Match($content, 'versionName\s+"([\d.]+)"').Groups[1].Value
$baseName = if ($publishedCode -gt $oldCode -and $publishedName -match '^\d+(\.\d+){1,2}$') { $publishedName } else { $oldName }
$parts = $baseName.Split('.')
if ($parts.Count -eq 2) { $parts += "0" }   # "1.0" → ["1","0","0"]
$parts[-1] = [string]([int]$parts[-1] + 1)   # 递增 PATCH
$newName = $parts -join '.'
$content = $content -replace "versionName\s+`"$([regex]::Escape($oldName))`"", "versionName `"$newName`""
[System.IO.File]::WriteAllText(
    $GradlePath,
    $content,
    (New-Object System.Text.UTF8Encoding($false))
)

$versionName = $newName

Write-Host "   versionCode: $oldCode → $newCode" -ForegroundColor Green
Write-Host "   versionName: $oldName → $newName" -ForegroundColor Green

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

# ── Step 4: Git commit + push ─────────────────────────────────────────────────

Write-Host "📤 提交版本号变动..." -ForegroundColor Cyan

# rustfmt（如有 .rs 改动）
$rs = @(git -C $RepoRoot diff --name-only) + @(git -C $RepoRoot ls-files --others --exclude-standard) |
    Where-Object { $_ -match '\.rs$' }
if ($rs) { rustfmt $rs }

git -C $RepoRoot add android/app/build.gradle
git -C $RepoRoot commit -m "release(android): v$versionName (build $newCode) - $Changelog"
if ($LASTEXITCODE -ne 0) { Write-Error "git commit 失败" }
Push-HeadToMain

$sha = git -C $RepoRoot rev-parse --short HEAD
$shaFull = (git -C $RepoRoot rev-parse HEAD).Trim()
Write-Host "   Commit SHA: $sha" -ForegroundColor Green

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
            Write-Host "   处理：" -ForegroundColor Yellow
            Write-Host "     1) 本次 build.gradle 版本号 commit 已推送，不需回退，git 历史无损失。" -ForegroundColor Yellow
            Write-Host "     2) 本次本地编译的 APK 作废，不上传。" -ForegroundColor Yellow
            Write-Host "     3) 如需本机中验证已发布的新版 APK，直接下载 $ServerUrl/app/ElonSpeed-latest.apk。" -ForegroundColor Yellow
            Write-Host "     4) 如确要覆盖（不推荐）：重跑加 -Force。" -ForegroundColor Yellow
            Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Yellow
            Write-Host ""
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

Write-Host ""
Write-Host ("=" * 60) -ForegroundColor Cyan
Write-Host "✅ 发布完成！" -ForegroundColor Green
Write-Host "   版本: v$versionName (build $newCode)" -ForegroundColor White
Write-Host "   SHA:  $sha" -ForegroundColor White
Write-Host "   下载: $downloadUrl" -ForegroundColor White
Write-Host ("=" * 60) -ForegroundColor Cyan
