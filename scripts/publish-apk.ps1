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

# ── Step 0: Git pull（防止 push 冲突） ────────────────────────────────────────

Write-Host "🔄 同步最新代码..." -ForegroundColor Cyan
git -C $RepoRoot pull --rebase origin main

# ── Step 1: 递增 versionCode，确认 versionName ────────────────────────────────

Write-Host "📝 更新版本号..." -ForegroundColor Cyan

$content = Get-Content $GradlePath -Raw

$oldCode = [int]([regex]::Match($content, 'versionCode\s+(\d+)').Groups[1].Value)
$newCode = $oldCode + 1
$content = $content -replace "versionCode\s+$oldCode", "versionCode $newCode"

# 自动递增 versionName PATCH（1.0 → 1.0.1，1.0.1 → 1.0.2，1.2 → 1.2.1）
$oldName = [regex]::Match($content, 'versionName\s+"([\d.]+)"').Groups[1].Value
$parts = $oldName.Split('.')
if ($parts.Count -eq 2) { $parts += "0" }   # "1.0" → ["1","0","0"]
$parts[-1] = [string]([int]$parts[-1] + 1)   # 递增 PATCH
$newName = $parts -join '.'
$content = $content -replace "versionName\s+`"$([regex]::Escape($oldName))`"", "versionName `"$newName`""
Set-Content $GradlePath $content -NoNewline

$versionName = $newName

Write-Host "   versionCode: $oldCode → $newCode" -ForegroundColor Green
Write-Host "   versionName: $oldName → $newName" -ForegroundColor Green

# ── Step 2: 编译 APK ─────────────────────────────────────────────────────────

if (-not $SkipBuild) {
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
git -C $RepoRoot push origin HEAD:main

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

# ── 汇报 ──────────────────────────────────────────────────────────────────────

Write-Host ""
Write-Host "=" * 60 -ForegroundColor Cyan
Write-Host "✅ 发布完成！" -ForegroundColor Green
Write-Host "   版本: v$versionName (build $newCode)" -ForegroundColor White
Write-Host "   SHA:  $sha" -ForegroundColor White
Write-Host "   下载: $downloadUrl" -ForegroundColor White
Write-Host "=" * 60 -ForegroundColor Cyan
