use crate::project_scaffold::ProjectScaffoldRequest;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

const WRAPPED_TOOLS: &[&str] = &[
    "npm", "npx", "pnpm", "yarn", "corepack", "rustup", "cargo", "curl", "wget", "git", "gradle",
    "gradlew",
];

pub(crate) fn ensure_project_download_router_files(
    repo: &Path,
    req: &ProjectScaffoldRequest<'_>,
) -> io::Result<()> {
    ensure_file(repo.join("scripts").join("elon-tool-router.ps1"), || {
        download_router_script(req)
    })?;
    ensure_file(repo.join("docs").join("download-router.md"), || {
        download_router_doc(req)
    })?;
    for tool in WRAPPED_TOOLS {
        ensure_file(
            repo.join("scripts")
                .join("tool-router-bin")
                .join(format!("{tool}.cmd")),
            || wrapper_script(tool),
        )?;
    }
    Ok(())
}

fn ensure_file(path: PathBuf, content: impl FnOnce() -> io::Result<String>) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content()?)
}

pub fn wrapper_script(tool: &str) -> io::Result<String> {
    Ok(format!(
        r#"@echo off
setlocal
set "ELON_ROUTER_WRAPPER_DIR=%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0..\elon-tool-router.ps1" run {tool} %*
exit /b %ERRORLEVEL%
"#
    ))
}

pub fn download_router_doc(req: &ProjectScaffoldRequest<'_>) -> io::Result<String> {
    Ok(format!(
        r#"# Elon Smart Download Router

This project includes a local command router for AI-driven builds and installs.

- project_id: `{}`
- template: `{}`
- default mode: `auto`
- scope: current Elon project/agent process only

The router prepends `scripts\tool-router-bin` to the process `PATH`. When an AI runs common tools such as `npm`, `rustup`, `cargo`, `curl`, or `gradle`, the wrapper picks a faster verified source or proxy setting, records a trace, and then delegates to the real tool.

It is fail-open by default. If routing logic fails, it prints a diagnostic line and falls back to the original command.

Useful commands:

```powershell
scripts\elon-tool-router.ps1 status
scripts\elon-tool-router.ps1 doctor
scripts\elon-tool-router.ps1 profile -Mode auto
scripts\elon-tool-router.ps1 profile -Disable
$env:ELON_ROUTER_BYPASS=1; npm ci
```

Traces are stored under `.elon\tool-router\traces`.
"#,
        req.project_id, req.template
    ))
}

pub fn download_router_script(_req: &ProjectScaffoldRequest<'_>) -> io::Result<String> {
    Ok(r#"
param(
    [ValidateSet('run', 'doctor', 'status', 'profile', 'audit')][string]$Action = 'status',
    [string]$Tool = '',
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$CommandArgs = @(),
    [ValidateSet('', 'auto', 'direct', 'system_proxy', 'off')][string]$Mode = '',
    [switch]$Enable,
    [switch]$Disable
)

$ErrorActionPreference = 'Stop'
$RouterVersion = '0.1'
$ProjectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$RouterHome = Join-Path $ProjectRoot '.elon\tool-router'
$ProjectProfilePath = Join-Path $RouterHome 'profile.json'
$TraceDir = Join-Path $RouterHome 'traces'
$CachePath = Join-Path $RouterHome 'cache.json'
$GlobalProfilePath = ''
if ($env:APPDATA) { $GlobalProfilePath = Join-Path $env:APPDATA 'elon-node-agent\download-router.json' }
elseif ($env:LOCALAPPDATA) { $GlobalProfilePath = Join-Path $env:LOCALAPPDATA 'Elon\download-router.json' }

function Ensure-RouterDirs {
    New-Item -ItemType Directory -Force -Path $RouterHome | Out-Null
    New-Item -ItemType Directory -Force -Path $TraceDir | Out-Null
}

function Read-JsonFile {
    param([string]$Path)
    if (-not $Path -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
    try { return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json } catch { return $null }
}

function New-DefaultProfile {
    [pscustomobject]@{
        enabled = $true
        mode = 'auto'
        failOpen = $true
        cacheMinutes = 60
        updatedAt = (Get-Date).ToUniversalTime().ToString('o')
    }
}

function Get-RouterProfile {
    $profile = New-DefaultProfile
    foreach ($path in @($GlobalProfilePath, $ProjectProfilePath)) {
        $loaded = Read-JsonFile $path
        if (-not $loaded) { continue }
        foreach ($prop in $loaded.PSObject.Properties) {
            $profile | Add-Member -NotePropertyName $prop.Name -NotePropertyValue $prop.Value -Force
        }
    }
    if ($profile.mode -notin @('auto', 'direct', 'system_proxy', 'off')) { $profile.mode = 'auto' }
    if ($profile.mode -eq 'off') { $profile.enabled = $false }
    return $profile
}

function Save-ProjectProfile {
    param($Profile)
    Ensure-RouterDirs
    $Profile.updatedAt = (Get-Date).ToUniversalTime().ToString('o')
    $Profile | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $ProjectProfilePath -Encoding UTF8
}

function Read-Cache {
    $cache = Read-JsonFile $CachePath
    if ($cache) { return $cache }
    return [pscustomobject]@{}
}

function Save-Cache {
    param($Cache)
    Ensure-RouterDirs
    $Cache | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $CachePath -Encoding UTF8
}

function Now-Millis {
    [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
}

function Write-RouterTrace {
    param([string]$ToolName, [string]$Phase, [AllowNull()]$Data)
    try {
        Ensure-RouterDirs
        $id = '{0:yyyyMMdd-HHmmssfff}-{1}-{2}.json' -f (Get-Date), $PID, ([Guid]::NewGuid().ToString('N').Substring(0, 8))
        $payload = [ordered]@{
            schema = 'elon.download-router.trace.v1'
            routerVersion = $RouterVersion
            ts = (Get-Date).ToUniversalTime().ToString('o')
            projectRoot = $ProjectRoot
            tool = $ToolName
            phase = $Phase
            data = $Data
        }
        $payload | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $TraceDir $id) -Encoding UTF8
    } catch {
        Write-Warning "download router trace failed: $($_.Exception.Message)"
    }
}

function Get-SystemProxyUrl {
    if ($env:HTTPS_PROXY) { return $env:HTTPS_PROXY }
    if ($env:HTTP_PROXY) { return $env:HTTP_PROXY }
    if (-not $IsWindows -and $PSVersionTable.PSVersion.Major -ge 6) { return '' }
    try {
        $proxy = Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings' -ErrorAction Stop
        if ($proxy.ProxyEnable -eq 1 -and $proxy.ProxyServer) {
            $value = [string]$proxy.ProxyServer
            $first = @($value -split ';' | Where-Object { $_ })[0]
            if ($first -match '=') { $first = (@($first -split '=')[-1]) }
            if ($first -notmatch '^[a-z]+://') { $first = "http://$first" }
            return $first
        }
    } catch {}
    return ''
}

function Invoke-Probe {
    param([string]$Name, [string]$Url, [string]$ProxyUrl)
    $started = Get-Date
    $ok = $false
    $err = ''
    try {
        $args = @{
            Uri = $Url
            Method = 'GET'
            TimeoutSec = 8
            UseBasicParsing = $true
            Headers = @{ Range = 'bytes=0-65535' }
        }
        if ($ProxyUrl) { $args.Proxy = $ProxyUrl }
        $resp = Invoke-WebRequest @args
        $ok = [int]$resp.StatusCode -ge 200 -and [int]$resp.StatusCode -lt 400
    } catch {
        $err = $_.Exception.Message
    }
    $elapsed = [int]((Get-Date) - $started).TotalMilliseconds
    [pscustomobject]@{ name = $Name; url = $Url; proxy = $ProxyUrl; ok = $ok; elapsedMs = $elapsed; error = $err }
}

function Candidate-List {
    param([string]$Kind, [string]$ProxyUrl)
    if ($Kind -eq 'rust') {
        return @(
            [pscustomobject]@{ name='rsproxy'; probe='https://rsproxy.cn/dist/channel-rust-stable.toml'; env=@{ RUSTUP_DIST_SERVER='https://rsproxy.cn'; RUSTUP_UPDATE_ROOT='https://rsproxy.cn/rustup' } },
            [pscustomobject]@{ name='tuna'; probe='https://mirrors.tuna.tsinghua.edu.cn/rustup/dist/channel-rust-stable.toml'; env=@{ RUSTUP_DIST_SERVER='https://mirrors.tuna.tsinghua.edu.cn/rustup'; RUSTUP_UPDATE_ROOT='https://mirrors.tuna.tsinghua.edu.cn/rustup/rustup' } },
            [pscustomobject]@{ name='ustc'; probe='https://mirrors.ustc.edu.cn/rust-static/dist/channel-rust-stable.toml'; env=@{ RUSTUP_DIST_SERVER='https://mirrors.ustc.edu.cn/rust-static'; RUSTUP_UPDATE_ROOT='https://mirrors.ustc.edu.cn/rust-static/rustup' } },
            [pscustomobject]@{ name='official'; probe='https://static.rust-lang.org/dist/channel-rust-stable.toml'; env=@{ RUSTUP_DIST_SERVER='https://static.rust-lang.org'; RUSTUP_UPDATE_ROOT='https://static.rust-lang.org/rustup' } }
        )
    }
    if ($Kind -eq 'npm') {
        return @(
            [pscustomobject]@{ name='npmmirror'; probe='https://registry.npmmirror.com/npm'; env=@{ npm_config_registry='https://registry.npmmirror.com' } },
            [pscustomobject]@{ name='official'; probe='https://registry.npmjs.org/npm'; env=@{ npm_config_registry='https://registry.npmjs.org' } }
        )
    }
    if ($Kind -eq 'generic' -and $ProxyUrl) {
        return @([pscustomobject]@{ name='system_proxy'; probe='https://github.com'; env=@{ HTTP_PROXY=$ProxyUrl; HTTPS_PROXY=$ProxyUrl } })
    }
    return @()
}

function Get-ToolKind {
    param([string]$ToolName)
    switch ($ToolName.ToLowerInvariant()) {
        { $_ -in @('rustup') } { return 'rust' }
        { $_ -in @('npm', 'npx', 'pnpm', 'yarn', 'corepack') } { return 'npm' }
        { $_ -in @('curl', 'wget', 'git') } { return 'generic' }
        default { return '' }
    }
}

function Select-Route {
    param([string]$Kind, $Profile)
    if (-not $Kind -or -not $Profile.enabled -or $Profile.mode -eq 'off') { return $null }
    $cache = Read-Cache
    $key = $Kind
    $cached = $cache.$key
    if ($cached -and ([int64]$cached.expiresAtMs -gt (Now-Millis))) { return $cached.choice }
    $proxyUrl = Get-SystemProxyUrl
    $candidates = Candidate-List $Kind $proxyUrl
    if ($candidates.Count -eq 0) { return $null }
    if ($Profile.mode -eq 'direct') {
        $choice = $candidates | Select-Object -First 1
    } elseif ($Profile.mode -eq 'system_proxy' -and $proxyUrl) {
        $choice = [pscustomobject]@{ name='system_proxy'; env=@{ HTTP_PROXY=$proxyUrl; HTTPS_PROXY=$proxyUrl } }
    } else {
        $results = foreach ($candidate in $candidates) {
            Invoke-Probe $candidate.name $candidate.probe ''
        }
        $winner = $results | Where-Object { $_.ok } | Sort-Object elapsedMs | Select-Object -First 1
        if ($winner) { $choice = $candidates | Where-Object { $_.name -eq $winner.name } | Select-Object -First 1 }
        else { $choice = $null }
        Write-RouterTrace $Kind 'probe' ([ordered]@{ mode=$Profile.mode; proxy=$proxyUrl; results=$results })
    }
    if ($choice) {
        $cache | Add-Member -NotePropertyName $key -NotePropertyValue ([pscustomobject]@{
            choice = $choice
            expiresAtMs = (Now-Millis) + ([int]$Profile.cacheMinutes * 60000)
        }) -Force
        Save-Cache $cache
    }
    return $choice
}

function Apply-Choice {
    param($Choice)
    if (-not $Choice -or -not $Choice.env) { return }
    if ($Choice.env -is [System.Collections.IDictionary]) {
        foreach ($key in $Choice.env.Keys) {
            [Environment]::SetEnvironmentVariable([string]$key, [string]$Choice.env[$key], 'Process')
        }
        return
    }
    foreach ($prop in $Choice.env.PSObject.Properties) {
        [Environment]::SetEnvironmentVariable($prop.Name, [string]$prop.Value, 'Process')
    }
}

function Resolve-RealTool {
    param([string]$ToolName)
    $wrapperDir = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'tool-router-bin')).Path
    $all = @(Get-Command $ToolName -All -ErrorAction SilentlyContinue)
    foreach ($cmd in $all) {
        $source = [string]$cmd.Source
        if ($source -and -not $source.StartsWith($wrapperDir, [System.StringComparison]::OrdinalIgnoreCase)) {
            return $source
        }
    }
    return ''
}

function Invoke-RealTool {
    param([string]$ToolName, [string[]]$Args)
    $real = Resolve-RealTool $ToolName
    if (-not $real) { throw "real tool not found: $ToolName" }
    & $real @Args
    return $LASTEXITCODE
}

function Invoke-RouterRun {
    if (-not $Tool) { throw 'Tool is required for run action.' }
    if ($env:ELON_ROUTER_INNER -eq '1' -or $env:ELON_ROUTER_BYPASS -eq '1') {
        exit (Invoke-RealTool $Tool $CommandArgs)
    }
    $env:ELON_ROUTER_INNER = '1'
    $profile = Get-RouterProfile
    $kind = Get-ToolKind $Tool
    try {
        $choice = Select-Route $kind $profile
        Apply-Choice $choice
        if ($Tool.ToLowerInvariant() -in @('gradle', 'gradlew')) {
            Ensure-GradleInit
            $CommandArgs = @('--init-script', (Gradle-InitPath)) + $CommandArgs
        }
        Write-RouterTrace $Tool 'run' ([ordered]@{ args=$CommandArgs; profile=$profile; kind=$kind; choice=$choice })
        exit (Invoke-RealTool $Tool $CommandArgs)
    } catch {
        Write-RouterTrace $Tool 'failed_open' ([ordered]@{ error=$_.Exception.Message; args=$CommandArgs })
        Write-Warning "[elon-router] $($_.Exception.Message); falling back to real $Tool"
        exit (Invoke-RealTool $Tool $CommandArgs)
    }
}

function Gradle-InitPath {
    Join-Path $RouterHome 'gradle-init.gradle'
}

function Ensure-GradleInit {
    Ensure-RouterDirs
    $path = Gradle-InitPath
    if (Test-Path -LiteralPath $path) { return }
    @"
settingsEvaluated { settings ->
  settings.pluginManagement.repositories {
    maven { url 'https://maven.aliyun.com/repository/gradle-plugin' }
    google(); mavenCentral(); gradlePluginPortal()
  }
}
allprojects {
  repositories {
    maven { url 'https://maven.aliyun.com/repository/google' }
    maven { url 'https://maven.aliyun.com/repository/maven-public' }
    google(); mavenCentral()
  }
}
"@ | Set-Content -LiteralPath $path -Encoding UTF8
}

function Invoke-Doctor {
    $profile = Get-RouterProfile
    $proxy = Get-SystemProxyUrl
    $rust = Candidate-List rust $proxy | ForEach-Object { Invoke-Probe $_.name $_.probe '' }
    $npm = Candidate-List npm $proxy | ForEach-Object { Invoke-Probe $_.name $_.probe '' }
    $payload = [ordered]@{
        ok = $true
        routerVersion = $RouterVersion
        profile = $profile
        profilePath = $ProjectProfilePath
        globalProfilePath = $GlobalProfilePath
        wrapperBin = (Join-Path $PSScriptRoot 'tool-router-bin')
        systemProxy = $proxy
        rust = $rust
        npm = $npm
        traceDir = $TraceDir
        bypass = 'ELON_ROUTER_BYPASS=1'
        failOpen = $true
    }
    Write-RouterTrace 'doctor' 'doctor' $payload
    $payload | ConvertTo-Json -Depth 20
}

function Invoke-Profile {
    $profile = Get-RouterProfile
    if ($Enable) { $profile.enabled = $true }
    if ($Disable) { $profile.enabled = $false; $profile.mode = 'off' }
    if ($Mode) {
        $profile.mode = $Mode
        $profile.enabled = $Mode -ne 'off'
    }
    if ($Enable -or $Disable -or $Mode) { Save-ProjectProfile $profile }
    $profile | ConvertTo-Json -Depth 8
}

function Invoke-Status {
    $profile = Get-RouterProfile
    [ordered]@{
        ok = $true
        routerVersion = $RouterVersion
        enabled = [bool]$profile.enabled
        mode = $profile.mode
        projectProfilePath = $ProjectProfilePath
        globalProfilePath = $GlobalProfilePath
        traceDir = $TraceDir
        wrapperBin = (Join-Path $PSScriptRoot 'tool-router-bin')
        bypass = 'ELON_ROUTER_BYPASS=1'
        failOpen = $true
    } | ConvertTo-Json -Depth 8
}

function Invoke-Audit {
    $patterns = @('npm ', 'rustup ', 'curl ', 'wget ', 'gradle ', 'gradlew')
    $files = Get-ChildItem -LiteralPath $ProjectRoot -Recurse -File -Include *.ps1,*.cmd,*.bat,*.sh,*.gradle,*.toml,package.json -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -notmatch '\\node_modules\\|\\target\\|\\.git\\|\\build\\|\\.gradle\\' }
    $hits = @()
    foreach ($file in $files) {
        $text = Get-Content -LiteralPath $file.FullName -Raw -ErrorAction SilentlyContinue
        foreach ($pattern in $patterns) {
            if ($text -like "*$pattern*") {
                $hits += [pscustomobject]@{ path = $file.FullName.Substring($ProjectRoot.Length).TrimStart('\'); pattern = $pattern.Trim() }
            }
        }
    }
    [ordered]@{ ok=$true; hits=$hits; recommendation='Prefer scripts\elon.ps1 or PATH wrapper entrypoints for AI tasks.' } | ConvertTo-Json -Depth 10
}

switch ($Action) {
    'run' { Invoke-RouterRun }
    'doctor' { Invoke-Doctor }
    'profile' { Invoke-Profile }
    'audit' { Invoke-Audit }
    default { Invoke-Status }
}
"#
    .trim_start()
    .to_string())
}
