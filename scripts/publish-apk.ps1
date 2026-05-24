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

    [switch]$SkipBuild
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

$DefaultKeystore = Join-Path $env:USERPROFILE ".elon\signing\elon-release.jks"
$LegacyKeystore  = Join-Path $AndroidDir "app\elon-release.jks"
$UserGradleProps = Join-Path $env:USERPROFILE ".gradle\gradle.properties"

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

function Push-HeadToMain {
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        git -C $RepoRoot push origin HEAD:main
        if ($LASTEXITCODE -eq 0) { return }

        Write-Warning "push 被拒绝，正在 fetch + rebase 后重试（第 $attempt 次）..."
        git -C $RepoRoot fetch origin
        if ($LASTEXITCODE -ne 0) { Write-Error "git fetch 失败" }
        git -C $RepoRoot rebase origin/main
        if ($LASTEXITCODE -ne 0) { Write-Error "git rebase 失败，请解决冲突后重试发布。" }
    }

    Write-Error "重试 3 次后仍无法推送，已停止上传 APK。"
}

# ── Step 0: Git pull（防止 push 冲突） ────────────────────────────────────────

Write-Host "🔄 同步最新代码..." -ForegroundColor Cyan
git -C $RepoRoot pull --rebase origin main
if ($LASTEXITCODE -ne 0) { Write-Error "git pull --rebase 失败" }

# ── Step 1: 递增 versionCode，确认 versionName ────────────────────────────────

Write-Host "📝 更新版本号..." -ForegroundColor Cyan

$content = Get-Content $GradlePath -Encoding UTF8 -Raw

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
    Write-Host "🔨 编译 Release APK..." -ForegroundColor Cyan
    Push-Location $AndroidDir
    try {
        .\gradlew.bat assembleRelease
    } finally {
        Pop-Location
    }
} else {
    Write-Host "⏭️  跳过编译（-SkipBuild）" -ForegroundColor Yellow
}

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
Write-Host "   Commit SHA: $sha" -ForegroundColor Green

# ── Step 5: 生成 version.json ─────────────────────────────────────────────────

$downloadUrl = "$ServerUrl/app/ElonSpeed-latest.apk"
$versionJson = @{
    versionCode = $newCode
    versionName = $versionName
    downloadUrl = $downloadUrl
    changelog   = $Changelog
    forceUpdate = $false
    fileSize    = $fileSize
} | ConvertTo-Json -Depth 2

$tmpJson = Join-Path $env:TEMP "elon-version.json"
$versionJson | Set-Content $tmpJson -Encoding UTF8
Write-Host "📋 version.json 已生成" -ForegroundColor Green

# ── Step 6: SCP 上传到服务器 ──────────────────────────────────────────────────

Write-Host "🚀 上传到服务器..." -ForegroundColor Cyan

# 确保目标目录存在
ssh -o ProxyCommand=none $ServerHost "mkdir -p $ServerDir"

# 上传 APK
scp -o ProxyCommand=none $apk.FullName "${ServerHost}:${ServerDir}/ElonSpeed-latest.apk"
Write-Host "   ✅ APK 上传完成" -ForegroundColor Green

# 上传 version.json
scp -o ProxyCommand=none $tmpJson "${ServerHost}:${ServerDir}/version.json"
Write-Host "   ✅ version.json 上传完成" -ForegroundColor Green

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
