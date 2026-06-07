#!/usr/bin/env pwsh
# setup-node-env.ps1 — 一龙 PC 节点 AI 编码工具安装脚本
#
# 作用：为成为一龙 AI 编码节点安装必要工具：
#   1. Git            （代码操作必需）
#   2. JDK 17         （Android Gradle 构建必需）
#   3. Node.js LTS    （Codex CLI 运行时）
#   4. Codex CLI      （OpenAI AI 编码代理：npm install -g @openai/codex）
#   5. Android SDK    （编译发布 APK 必需：cmdline-tools + platforms-34 + build-tools-34）
#   6. Gradle 镜像    （阿里云，国内大幅加速 Android 构建）
#   7. Ollama         （可选，本地 LLM 推理，贡献算力赚积分）
#   8. OPENAI_API_KEY （持久化到 node-agent.env + 用户环境变量）
#
# 用法：
#   .\scripts\setup-node-env.ps1
#   .\scripts\setup-node-env.ps1 -ApiKey "sk-..."   # 跳过交互，直接保存 API Key
#   .\scripts\setup-node-env.ps1 -NoOllama          # 跳过 Ollama 安装
#   .\scripts\setup-node-env.ps1 -Silent            # 非交互模式（由管理页触发）
#
# 也可在 elon-node-agent 管理页（http://127.0.0.1:7799/）点「一键安装」自动触发。

param(
    [string]$ApiKey  = $env:OPENAI_API_KEY,
    [string]$EnvFile = "",
    [switch]$NoOllama,
    [switch]$Silent   # 非交互：跳过所有 Read-Host，仅安装，不询问
)

$ErrorActionPreference = 'Continue'

# ── 颜色辅助 ──────────────────────────────────────────────────────────────────
function Step([string]$msg)  { Write-Host "`n▶ $msg" -ForegroundColor Cyan }
function Ok([string]$msg)    { Write-Host "  ✓ $msg" -ForegroundColor Green }
function Warn([string]$msg)  { Write-Host "  ⚠ $msg" -ForegroundColor Yellow }
function Err([string]$msg)   { Write-Host "  ✗ $msg" -ForegroundColor Red }

function Has([string]$cmd) {
    $null -ne (Get-Command $cmd -ErrorAction SilentlyContinue)
}

# winget 静默安装（已安装则跳过，返回 $true 表示成功/已存在）
function WingetInstall([string]$id, [string]$label) {
    if (-not (Has 'winget')) {
        Warn "winget 不可用。请手动安装 ${label}。"
        return $false
    }
    Step "安装 $label (winget install --id $id)"
    winget install --id $id --silent --accept-source-agreements --accept-package-agreements
    # -1978335189 = APPINSTALLER_ERROR_ALREADY_INSTALLED（已安装，视为成功）
    return ($LASTEXITCODE -eq 0 -or $LASTEXITCODE -eq -1978335189)
}

# 刷新当前进程 PATH（安装后立即生效，无需重启终端）
function Refresh-Path {
    $machinePath = [System.Environment]::GetEnvironmentVariable('PATH', 'Machine')
    $userPath    = [System.Environment]::GetEnvironmentVariable('PATH', 'User')
    $env:PATH    = "$machinePath;$userPath"
    # 同步 ANDROID_HOME 等可能刚写入的用户变量
    $ah = [System.Environment]::GetEnvironmentVariable('ANDROID_HOME', 'User')
    if ($ah) { $env:ANDROID_HOME = $ah }
    $ah2 = [System.Environment]::GetEnvironmentVariable('ANDROID_SDK_ROOT', 'User')
    if ($ah2) { $env:ANDROID_SDK_ROOT = $ah2 }
}

Write-Host "═══════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  一龙 PC 节点 — AI 编码环境安装向导" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════" -ForegroundColor Cyan

# ── 1. Git ────────────────────────────────────────────────────────────────────
Step "检查 Git"
if (Has 'git') {
    Ok "Git 已安装：$(git --version)"
} else {
    if (WingetInstall 'Git.Git' 'Git') {
        Refresh-Path
        if (Has 'git') { Ok "Git 安装成功：$(git --version)" }
        else { Warn "Git 安装完毕，请重启终端后验证（git --version）" }
    }
}

# ── 2. JDK 17（Android Gradle 构建必需）─────────────────────────────────────
Step "检查 JDK 17"
$javaOk = Has 'java'
if ($javaOk) {
    $javaVer = java -version 2>&1 | Select-Object -First 1
    Ok "JDK 已安装：$javaVer"
} else {
    # 优先尝试 Eclipse Adoptium Temurin 17（主流 Android 推荐）
    if (WingetInstall 'EclipseAdoptium.Temurin.17.JDK' 'JDK 17 (Temurin)') {
        Refresh-Path
        if (Has 'java') { Ok "JDK 安装成功：$(java -version 2>&1 | Select-Object -First 1)" }
        else { Warn "JDK 安装完毕，请重启终端后验证（java -version）" }
    }
}

# ── 3. Node.js LTS ────────────────────────────────────────────────────────────
Step "检查 Node.js"
if (Has 'node') {
    Ok "Node.js 已安装：$(node --version)"
} else {
    if (WingetInstall 'OpenJS.NodeJS.LTS' 'Node.js LTS') {
        Refresh-Path
        if (Has 'node') { Ok "Node.js 安装成功：$(node --version)" }
        else { Warn "Node.js 安装完毕，请重启终端后验证（node --version）" }
    }
}

# ── 4. Codex CLI ──────────────────────────────────────────────────────────────
Step "检查 Codex CLI (@openai/codex)"
if (Has 'codex') {
    Ok "Codex CLI 已安装"
} elseif (Has 'npm') {
    Step "安装 Codex CLI（npm install -g @openai/codex）"
    # 先尝试国内镜像，失败再回落官方源
    npm install -g @openai/codex --registry https://registry.npmmirror.com 2>$null
    if ($LASTEXITCODE -ne 0) {
        Warn "国内镜像失败，尝试官方源..."
        npm install -g @openai/codex
    }
    Refresh-Path
    if (Has 'codex') { Ok "Codex CLI 安装成功" }
    else { Warn "Codex CLI 安装完毕，可能需要重启终端后验证（codex --version）" }
} else {
    Warn "npm 不可用，跳过 Codex CLI 安装。请先成功安装 Node.js 后重新运行本脚本。"
}

# ── 5. Android SDK ────────────────────────────────────────────────────────────
Step "检查 Android SDK（APK 编译必需）"

# 确定 ANDROID_HOME 路径（优先已有的环境变量）
$androidHome = $env:ANDROID_HOME
if (-not $androidHome) { $androidHome = $env:ANDROID_SDK_ROOT }
if (-not $androidHome) { $androidHome = "$env:LOCALAPPDATA\Android\Sdk" }

$cmdlineToolsBin = "$androidHome\cmdline-tools\latest\bin"
$sdkmanager      = "$cmdlineToolsBin\sdkmanager.bat"
$platformsOk     = Test-Path "$androidHome\platforms\android-34"
$buildToolsOk    = Test-Path "$androidHome\build-tools\34.0.0"
$sdkManagerOk    = Test-Path $sdkmanager

if ($platformsOk -and $buildToolsOk) {
    Ok "Android SDK 已就绪（ANDROID_HOME=$androidHome）"
} else {
    # -- 5a. 下载 cmdline-tools（如未安装）
    if (-not $sdkManagerOk) {
        Step "下载 Android cmdline-tools（~130MB）"
        New-Item -ItemType Directory -Path "$androidHome\cmdline-tools" -Force | Out-Null
        $zipUrl  = "https://dl.google.com/android/repository/commandlinetools-win-11076708_latest.zip"
        $zipPath = "$env:TEMP\android-cmdline-tools.zip"
        try {
            Write-Host "  下载中（Google 官方，如慢可考虑挂代理后重新运行）..." -ForegroundColor Gray
            [System.Net.WebClient]::new().DownloadFile($zipUrl, $zipPath)
            # 解压，官方包内层目录名是 cmdline-tools，需重命名为 latest
            Expand-Archive -Path $zipPath -DestinationPath "$androidHome\cmdline-tools" -Force
            $extracted = "$androidHome\cmdline-tools\cmdline-tools"
            $latest    = "$androidHome\cmdline-tools\latest"
            if (Test-Path $extracted) {
                if (Test-Path $latest) { Remove-Item $latest -Recurse -Force }
                Rename-Item $extracted $latest
            }
            Remove-Item $zipPath -ErrorAction SilentlyContinue
            $sdkManagerOk = Test-Path $sdkmanager
            if ($sdkManagerOk) { Ok "cmdline-tools 解压完成" }
            else { Warn "cmdline-tools 解压后未找到 sdkmanager，请手动检查 $cmdlineToolsBin" }
        } catch {
            Warn "下载 cmdline-tools 失败：$_"
            Warn "请手动下载：https://developer.android.com/studio#command-line-tools-only"
        }
    } else {
        Ok "sdkmanager 已存在"
    }

    # -- 5b. 设置 ANDROID_HOME 环境变量
    [System.Environment]::SetEnvironmentVariable('ANDROID_HOME',     $androidHome, 'User')
    [System.Environment]::SetEnvironmentVariable('ANDROID_SDK_ROOT', $androidHome, 'User')
    $env:ANDROID_HOME     = $androidHome
    $env:ANDROID_SDK_ROOT = $androidHome
    Ok "ANDROID_HOME=$androidHome 已写入用户环境变量"

    # -- 5c. 把 cmdline-tools/latest/bin 和 platform-tools 加入用户 PATH
    $userPath = [System.Environment]::GetEnvironmentVariable('PATH', 'User') ?? ''
    $newPaths = @($cmdlineToolsBin, "$androidHome\platform-tools")
    foreach ($p in $newPaths) {
        if ($userPath -notlike "*$p*") { $userPath = "$userPath;$p" }
    }
    [System.Environment]::SetEnvironmentVariable('PATH', $userPath, 'User')

    # -- 5d. 用 sdkmanager 安装 SDK 组件
    if ($sdkManagerOk) {
        Step "安装 Android SDK 组件（platforms;android-34 + build-tools;34.0.0 + platform-tools）"
        # 接受全部 License（echo y 在 Windows 上用 "y" 管道）
        $licCmd = "echo y | `"$sdkmanager`" --sdk_root=`"$androidHome`" --licenses"
        cmd /c $licCmd 2>$null
        & $sdkmanager --sdk_root="$androidHome" "platforms;android-34" "build-tools;34.0.0" "platform-tools"
        if ($LASTEXITCODE -eq 0) { Ok "SDK 组件安装完成" }
        else { Warn "SDK 组件安装可能未完全成功，请手动运行：`n  $sdkmanager `"platforms;android-34`" `"build-tools;34.0.0`"" }
    }
    Refresh-Path
}

# ── 6. Gradle 阿里云镜像（国内加速，一次性配置）─────────────────────────────
Step "配置 Gradle 阿里云镜像（首次编译大幅提速）"

$gradleDir = "$HOME\.gradle"
$initFile  = "$gradleDir\init.gradle"
$propsFile = "$gradleDir\gradle.properties"

if (-not (Test-Path $gradleDir)) { New-Item -ItemType Directory -Path $gradleDir -Force | Out-Null }

if (-not (Test-Path $initFile) -or -not ((Get-Content $initFile -Raw) -match "maven.aliyun.com")) {
    Set-Content $initFile -Encoding UTF8 @'
// buildscript classpath（插件解析）走 allprojects.buildscript
allprojects {
    buildscript {
        repositories {
            maven { url "https://maven.aliyun.com/repository/google" }
            maven { url "https://maven.aliyun.com/repository/central" }
            maven { url "https://maven.aliyun.com/repository/gradle-plugin" }
            maven { url "https://maven.aliyun.com/repository/public" }
        }
    }
}
// 依赖仓库通过 settingsEvaluated 注入，避免与 FAIL_ON_PROJECT_REPOS 冲突
settingsEvaluated { settings ->
    settings.dependencyResolutionManagement {
        repositories {
            maven { url "https://maven.aliyun.com/repository/google" }
            maven { url "https://maven.aliyun.com/repository/central" }
            maven { url "https://maven.aliyun.com/repository/gradle-plugin" }
            maven { url "https://maven.aliyun.com/repository/public" }
        }
    }
}
'@
    Ok "~/.gradle/init.gradle 阿里云镜像写入完成"
} else {
    Ok "Gradle 阿里云镜像已配置"
}

# 禁用 JVM 系统代理（避免 JVM 读取系统 SOCKS 代理导致超时）
if (Test-Path $propsFile) {
    $propsContent = Get-Content $propsFile -Raw
} else {
    $propsContent = ""
}
if ($propsContent -notmatch "useSystemProxies") {
    Add-Content $propsFile "`nsystemProp.java.net.useSystemProxies=false" -Encoding UTF8
    Ok "~/.gradle/gradle.properties 已禁用 JVM 系统代理"
}

# ── 7. Ollama（可选）─────────────────────────────────────────────────────────
if (-not $NoOllama) {
    Step "检查 Ollama（本地 LLM，可选）"
    if (Has 'ollama') {
        Ok "Ollama 已安装：$(ollama --version 2>&1 | Select-Object -First 1)"
    } else {
        $doInstall = $true
        if (-not $Silent) {
            $yn = Read-Host "  是否安装 Ollama（让本节点贡献本地 LLM 算力赚积分）？[Y/n]"
            $doInstall = ($yn -eq '' -or $yn.ToLower().StartsWith('y'))
        }
        if ($doInstall) {
            if (WingetInstall 'Ollama.Ollama' 'Ollama') {
                Refresh-Path
                if (Has 'ollama') { Ok "Ollama 安装成功" }
                else { Warn "Ollama 安装完毕，请重启终端后验证（ollama --version）" }
            }
        } else {
            Ok "跳过 Ollama 安装（可后续手动安装：winget install Ollama.Ollama）"
        }
    }
}

# ── 8. OpenAI API Key ─────────────────────────────────────────────────────────
Step "配置 OpenAI API Key（Codex CLI 必需）"

if (-not $ApiKey -and -not $Silent) {
    Write-Host "  Codex CLI 需要 OpenAI API Key 才能运行 AI 编码任务。" -ForegroundColor Gray
    Write-Host "  可前往 https://platform.openai.com/api-keys 获取。" -ForegroundColor Gray
    $ApiKey = Read-Host "  请输入 OpenAI API Key（sk-...，直接回车跳过）"
    $ApiKey = $ApiKey.Trim()
}

if ($ApiKey -and $ApiKey.StartsWith('sk-')) {
    # 持久化到用户级环境变量
    [System.Environment]::SetEnvironmentVariable('OPENAI_API_KEY', $ApiKey, 'User')
    $env:OPENAI_API_KEY = $ApiKey
    Ok "OPENAI_API_KEY 已写入用户环境变量"

    # 同时写入 node-agent.env
    if (-not $EnvFile) {
        $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
        $candidates = @(
            (Join-Path $scriptDir 'node-agent-launcher\node-agent.env'),
            (Join-Path $scriptDir '..\scripts\node-agent-launcher\node-agent.env'),
            (Join-Path $scriptDir 'node-agent.env')
        )
        foreach ($c in $candidates) {
            if (Test-Path $c) { $EnvFile = $c; break }
        }
        if (-not $EnvFile) {
            $EnvFile = Join-Path $scriptDir 'node-agent-launcher\node-agent.env'
        }
    }

    if (Test-Path $EnvFile) {
        $lines  = Get-Content $EnvFile -Raw
        $found  = $false
        $result = ($lines -split "`n") | ForEach-Object {
            if ($_ -match '^#?\s*OPENAI_API_KEY=') { $found = $true; "OPENAI_API_KEY=$ApiKey" }
            else { $_ }
        }
        if (-not $found) { $result += "OPENAI_API_KEY=$ApiKey" }
        $result -join "`n" | Set-Content $EnvFile -Encoding UTF8 -NoNewline
    } else {
        $dir = Split-Path -Parent $EnvFile
        if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
        "OPENAI_API_KEY=$ApiKey`n" | Set-Content $EnvFile -Encoding UTF8 -NoNewline
    }
    Ok "OPENAI_API_KEY 已写入 $EnvFile"

} elseif ($ApiKey) {
    Warn "API Key 格式不正确（需以 sk- 开头），已跳过"
} else {
    Warn "未提供 OpenAI API Key。Codex CLI 将无法运行，请稍后在管理页手动填写。"
}

# ── 汇总 ──────────────────────────────────────────────────────────────────────
Write-Host "`n═══════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  安装完成！当前环境状态：" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════" -ForegroundColor Cyan

Refresh-Path

$androidSdkOk = (Test-Path "$env:ANDROID_HOME\platforms\android-34") -or
                (Test-Path "$env:ANDROID_SDK_ROOT\platforms\android-34") -or
                (Test-Path "$env:LOCALAPPDATA\Android\Sdk\platforms\android-34")
$gradleMirrorOk = (Test-Path "$HOME\.gradle\init.gradle") -and
                  ((Get-Content "$HOME\.gradle\init.gradle" -Raw) -match "maven.aliyun.com")

$checks = @(
    [pscustomobject]@{ Name = 'Git';             Ok = (Has 'git');        Note = '代码操作必需' }
    [pscustomobject]@{ Name = 'JDK 17';          Ok = (Has 'java');       Note = 'Android Gradle 构建必需' }
    [pscustomobject]@{ Name = 'Node.js';         Ok = (Has 'node');       Note = 'Codex 运行时' }
    [pscustomobject]@{ Name = 'npm';             Ok = (Has 'npm');        Note = 'Node 包管理器' }
    [pscustomobject]@{ Name = 'Codex CLI';       Ok = (Has 'codex');      Note = 'AI 编码代理' }
    [pscustomobject]@{ Name = 'Android SDK';     Ok = $androidSdkOk;      Note = 'APK 编译必需' }
    [pscustomobject]@{ Name = 'Gradle 镜像';      Ok = $gradleMirrorOk;    Note = '国内加速（阿里云）' }
    [pscustomobject]@{ Name = 'Ollama';          Ok = (Has 'ollama');     Note = '本地 LLM（可选）' }
    [pscustomobject]@{ Name = 'OPENAI_API_KEY';
        Ok   = ($env:OPENAI_API_KEY -and $env:OPENAI_API_KEY.StartsWith('sk-'));
        Note = 'Codex 鉴权' }
)

foreach ($c in $checks) {
    $sym = if ($c.Ok) { '✓' } else { '✗' }
    $col = if ($c.Ok) { 'Green' } else { 'Yellow' }
    Write-Host ("  {0} {1,-20} {2}" -f $sym, $c.Name, $c.Note) -ForegroundColor $col
}

Write-Host "`n重启 elon-node-agent 后即可开始接受 AI 编码任务。" -ForegroundColor Cyan
Write-Host "管理页：http://127.0.0.1:7799/" -ForegroundColor Gray

if (-not $Silent) {
    Write-Host "`n按任意键关闭..." -ForegroundColor DarkGray
    $null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
}
#
# 作用：为成为一龙 AI 编码节点安装必要工具：
#   1. Git          （代码操作必需）
#   2. Node.js LTS  （Codex CLI 运行时）
#   3. Codex CLI    （OpenAI AI 编码代理：npm install -g @openai/codex）
#   4. Ollama       （可选，本地 LLM 推理，贡献算力赚积分）
#   5. OPENAI_API_KEY（持久化到 node-agent.env + 用户环境变量）
#
# 用法：
#   .\scripts\setup-node-env.ps1
#   .\scripts\setup-node-env.ps1 -ApiKey "sk-..."     # 跳过交互，直接保存 API Key
#   .\scripts\setup-node-env.ps1 -NoOllama            # 跳过 Ollama 安装
#   .\scripts\setup-node-env.ps1 -Silent              # 非交互模式（由管理页触发）
#
# 也可在 elon-node-agent 管理页（http://127.0.0.1:7799/）点「一键安装」自动触发。

param(
    [string]$ApiKey  = $env:OPENAI_API_KEY,
    [string]$EnvFile = "",
    [switch]$NoOllama,
    [switch]$Silent   # 非交互：跳过所有 Read-Host，仅安装，不询问
)

$ErrorActionPreference = 'Continue'

# ── 颜色辅助 ──────────────────────────────────────────────────────────────────
function Step([string]$msg)  { Write-Host "`n▶ $msg" -ForegroundColor Cyan }
function Ok([string]$msg)    { Write-Host "  ✓ $msg" -ForegroundColor Green }
function Warn([string]$msg)  { Write-Host "  ⚠ $msg" -ForegroundColor Yellow }
function Err([string]$msg)   { Write-Host "  ✗ $msg" -ForegroundColor Red }

function Has([string]$cmd) {
    $null -ne (Get-Command $cmd -ErrorAction SilentlyContinue)
}

# winget 静默安装（已安装则跳过，返回 $true 表示成功/已存在）
function WingetInstall([string]$id, [string]$label) {
    if (-not (Has 'winget')) {
        Warn "winget 不可用。请手动安装 ${label}。"
        return $false
    }
    Step "安装 $label (winget install --id $id)"
    winget install --id $id --silent --accept-source-agreements --accept-package-agreements
    # -1978335189 = APPINSTALLER_ERROR_ALREADY_INSTALLED（已安装，视为成功）
    return ($LASTEXITCODE -eq 0 -or $LASTEXITCODE -eq -1978335189)
}

# 刷新当前进程 PATH（安装后立即生效，无需重启终端）
function Refresh-Path {
    $machinePath = [System.Environment]::GetEnvironmentVariable('PATH', 'Machine')
    $userPath    = [System.Environment]::GetEnvironmentVariable('PATH', 'User')
    $env:PATH    = "$machinePath;$userPath"
}

Write-Host "═══════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  一龙 PC 节点 — AI 编码环境安装向导" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════" -ForegroundColor Cyan

# ── 1. Git ────────────────────────────────────────────────────────────────────
Step "检查 Git"
if (Has 'git') {
    Ok "Git 已安装：$(git --version)"
} else {
    if (WingetInstall 'Git.Git' 'Git') {
        Refresh-Path
        if (Has 'git') { Ok "Git 安装成功：$(git --version)" }
        else { Warn "Git 安装完毕，请重启终端后验证（git --version）" }
    }
}

# ── 2. Node.js LTS ────────────────────────────────────────────────────────────
Step "检查 Node.js"
if (Has 'node') {
    Ok "Node.js 已安装：$(node --version)"
} else {
    if (WingetInstall 'OpenJS.NodeJS.LTS' 'Node.js LTS') {
        Refresh-Path
        if (Has 'node') { Ok "Node.js 安装成功：$(node --version)" }
        else { Warn "Node.js 安装完毕，请重启终端后验证（node --version）" }
    }
}

# ── 3. Codex CLI ──────────────────────────────────────────────────────────────
Step "检查 Codex CLI (@openai/codex)"
if (Has 'codex') {
    Ok "Codex CLI 已安装"
} elseif (Has 'npm') {
    Step "安装 Codex CLI（npm install -g @openai/codex）"
    # 先尝试国内镜像，失败再回落官方源
    npm install -g @openai/codex --registry https://registry.npmmirror.com 2>$null
    if ($LASTEXITCODE -ne 0) {
        Warn "国内镜像失败，尝试官方源..."
        npm install -g @openai/codex
    }
    Refresh-Path
    if (Has 'codex') { Ok "Codex CLI 安装成功" }
    else { Warn "Codex CLI 安装完毕，可能需要重启终端后验证（codex --version）" }
} else {
    Warn "npm 不可用，跳过 Codex CLI 安装。请先成功安装 Node.js 后重新运行本脚本。"
}

# ── 4. Ollama（可选）─────────────────────────────────────────────────────────
if (-not $NoOllama) {
    Step "检查 Ollama（本地 LLM，可选）"
    if (Has 'ollama') {
        Ok "Ollama 已安装：$(ollama --version 2>&1 | Select-Object -First 1)"
    } else {
        $doInstall = $true
        if (-not $Silent) {
            $yn = Read-Host "  是否安装 Ollama（让本节点贡献本地 LLM 算力赚积分）？[Y/n]"
            $doInstall = ($yn -eq '' -or $yn.ToLower().StartsWith('y'))
        }
        if ($doInstall) {
            if (WingetInstall 'Ollama.Ollama' 'Ollama') {
                Refresh-Path
                if (Has 'ollama') { Ok "Ollama 安装成功" }
                else { Warn "Ollama 安装完毕，请重启终端后验证（ollama --version）" }
            }
        } else {
            Ok "跳过 Ollama 安装（可后续手动安装：winget install Ollama.Ollama）"
        }
    }
}

# ── 5. OpenAI API Key ─────────────────────────────────────────────────────────
Step "配置 OpenAI API Key（Codex CLI 必需）"

if (-not $ApiKey -and -not $Silent) {
    Write-Host "  Codex CLI 需要 OpenAI API Key 才能运行 AI 编码任务。" -ForegroundColor Gray
    Write-Host "  可前往 https://platform.openai.com/api-keys 获取。" -ForegroundColor Gray
    $ApiKey = Read-Host "  请输入 OpenAI API Key（sk-...，直接回车跳过）"
    $ApiKey = $ApiKey.Trim()
}

if ($ApiKey -and $ApiKey.StartsWith('sk-')) {
    # 持久化到用户级环境变量（当前用户，重启后仍有效）
    [System.Environment]::SetEnvironmentVariable('OPENAI_API_KEY', $ApiKey, 'User')
    $env:OPENAI_API_KEY = $ApiKey
    Ok "OPENAI_API_KEY 已写入用户环境变量"

    # 同时写入 node-agent.env（node agent 重启后从此文件加载）
    if (-not $EnvFile) {
        # 优先检查脚本同目录的 node-agent-launcher/node-agent.env
        $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
        $candidates = @(
            (Join-Path $scriptDir 'node-agent-launcher\node-agent.env'),
            (Join-Path $scriptDir '..\scripts\node-agent-launcher\node-agent.env'),
            (Join-Path $scriptDir 'node-agent.env')
        )
        foreach ($c in $candidates) {
            if (Test-Path $c) { $EnvFile = $c; break }
        }
        # 如果都不存在，创建在 node-agent-launcher/ 下
        if (-not $EnvFile) {
            $EnvFile = Join-Path $scriptDir 'node-agent-launcher\node-agent.env'
        }
    }

    # 更新或追加 OPENAI_API_KEY= 行
    if (Test-Path $EnvFile) {
        $lines  = Get-Content $EnvFile -Raw
        $found  = $false
        $result = ($lines -split "`n") | ForEach-Object {
            if ($_ -match '^#?\s*OPENAI_API_KEY=') { $found = $true; "OPENAI_API_KEY=$ApiKey" }
            else { $_ }
        }
        if (-not $found) { $result += "OPENAI_API_KEY=$ApiKey" }
        $result -join "`n" | Set-Content $EnvFile -Encoding UTF8 -NoNewline
    } else {
        $dir = Split-Path -Parent $EnvFile
        if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
        "OPENAI_API_KEY=$ApiKey`n" | Set-Content $EnvFile -Encoding UTF8 -NoNewline
    }
    Ok "OPENAI_API_KEY 已写入 $EnvFile"

} elseif ($ApiKey) {
    Warn "API Key 格式不正确（需以 sk- 开头），已跳过"
} else {
    Warn "未提供 OpenAI API Key。Codex CLI 将无法运行，请稍后在管理页手动填写。"
}

# ── 汇总 ──────────────────────────────────────────────────────────────────────
Write-Host "`n═══════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  安装完成！当前环境状态：" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════" -ForegroundColor Cyan

Refresh-Path   # 最终刷新一次 PATH

$checks = @(
    [pscustomobject]@{ Name = 'Git';           Ok = (Has 'git');   Note = '代码操作必需' }
    [pscustomobject]@{ Name = 'Node.js';       Ok = (Has 'node');  Note = 'Codex 运行时' }
    [pscustomobject]@{ Name = 'npm';           Ok = (Has 'npm');   Note = 'Node 包管理器' }
    [pscustomobject]@{ Name = 'Codex CLI';     Ok = (Has 'codex'); Note = 'AI 编码代理' }
    [pscustomobject]@{ Name = 'Ollama';        Ok = (Has 'ollama');Note = '本地 LLM（可选）' }
    [pscustomobject]@{ Name = 'OPENAI_API_KEY';
        Ok   = ($env:OPENAI_API_KEY -and $env:OPENAI_API_KEY.StartsWith('sk-'));
        Note = 'Codex 鉴权' }
)

foreach ($c in $checks) {
    $sym = if ($c.Ok) { '✓' } else { '✗' }
    $col = if ($c.Ok) { 'Green' } else { 'Yellow' }
    Write-Host ("  {0} {1,-20} {2}" -f $sym, $c.Name, $c.Note) -ForegroundColor $col
}

Write-Host "`n重启 elon-node-agent 后即可开始接受 AI 编码任务。" -ForegroundColor Cyan
Write-Host "管理页：http://127.0.0.1:7799/" -ForegroundColor Gray

if (-not $Silent) {
    Write-Host "`n按任意键关闭..." -ForegroundColor DarkGray
    $null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
}
