#!/usr/bin/env powershell

[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [switch]$Apply,
    [switch]$IncludeExpiredTemp,
    [ValidateRange(7, 3650)]
    [int]$MinAgeDays = 30
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

function Get-NormalizedFullPath {
    param([Parameter(Mandatory = $true)][string]$Path)
    return [System.IO.Path]::GetFullPath($Path).TrimEnd([char[]]@('\', '/'))
}

function Test-PathWithinRoot {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Root
    )
    $pathKey = (Get-NormalizedFullPath $Path).ToLowerInvariant()
    $rootKey = (Get-NormalizedFullPath $Root).ToLowerInvariant()
    return ($pathKey -eq $rootKey -or $pathKey.StartsWith($rootKey + [System.IO.Path]::DirectorySeparatorChar))
}

function Assert-SafeRoot {
    param([Parameter(Mandatory = $true)][string]$Root)
    $full = Get-NormalizedFullPath $Root
    $volumeRoot = [System.IO.Path]::GetPathRoot($full).TrimEnd([char[]]@('\', '/'))
    if ($full -eq $volumeRoot) {
        throw "拒绝使用磁盘根作为清理候选根：$full"
    }
    if (Test-Path -LiteralPath $full) {
        $item = Get-Item -LiteralPath $full -Force
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "拒绝使用重解析点作为清理候选根：$full"
        }
    }
    return $full
}

function Test-TreeHasReparsePoint {
    param([Parameter(Mandatory = $true)][string]$Path)
    $pending = New-Object 'System.Collections.Generic.Stack[string]'
    $pending.Push((Get-NormalizedFullPath $Path))
    while ($pending.Count -gt 0) {
        $current = $pending.Pop()
        $item = Get-Item -LiteralPath $current -Force
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            return $true
        }
        if (-not $item.PSIsContainer) { continue }
        foreach ($child in Get-ChildItem -LiteralPath $current -Force -ErrorAction Stop) {
            if (($child.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                return $true
            }
            if ($child.PSIsContainer) { $pending.Push($child.FullName) }
        }
    }
    return $false
}

function Get-CandidateBytes {
    param([Parameter(Mandatory = $true)][string]$Path)
    [uint64]$total = 0
    $pending = New-Object 'System.Collections.Generic.Stack[string]'
    $pending.Push((Get-NormalizedFullPath $Path))
    while ($pending.Count -gt 0) {
        $current = $pending.Pop()
        $item = Get-Item -LiteralPath $current -Force
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "统计期间发现重解析点，拒绝继续：$current"
        }
        if (-not $item.PSIsContainer) {
            $total += [uint64]$item.Length
            continue
        }
        foreach ($child in Get-ChildItem -LiteralPath $current -Force -ErrorAction Stop) {
            if (($child.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "统计期间发现重解析点，拒绝继续：$($child.FullName)"
            }
            if ($child.PSIsContainer) {
                $pending.Push($child.FullName)
            } else {
                $total += [uint64]$child.Length
            }
        }
    }
    return $total
}

function New-Candidate {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$AllowedRoot,
        [Parameter(Mandatory = $true)][string]$Kind,
        [Parameter(Mandatory = $true)][string]$Reason
    )
    if (-not (Test-Path -LiteralPath $Path)) { return $null }
    [pscustomobject]@{
        Path = Get-NormalizedFullPath $Path
        AllowedRoot = Get-NormalizedFullPath $AllowedRoot
        Kind = $Kind
        Reason = $Reason
    }
}

function Assert-SafeCandidate {
    param([Parameter(Mandatory = $true)]$Candidate)
    $path = Get-NormalizedFullPath $Candidate.Path
    $root = Assert-SafeRoot $Candidate.AllowedRoot
    if ($path -eq $root -or -not (Test-PathWithinRoot -Path $path -Root $root)) {
        throw "候选路径越界，拒绝处理：$path（允许根：$root）"
    }
    if (Test-TreeHasReparsePoint $path) {
        throw "候选路径包含重解析点，拒绝处理：$path"
    }
}

if (-not $env:LOCALAPPDATA) {
    throw 'LOCALAPPDATA 不可用，无法定位一龙历史构建缓存。'
}

$buildTargetRoot = Assert-SafeRoot (Join-Path $env:LOCALAPPDATA 'Elon\build-target')
$tempRoot = $null
if ($IncludeExpiredTemp) {
    $tempRoot = Assert-SafeRoot ([System.IO.Path]::GetTempPath())
}

$candidates = @()
foreach ($name in @('elon-dev-cargo', 'elon-node-agent')) {
    $candidate = New-Candidate `
        -Path (Join-Path $buildTargetRoot $name) `
        -AllowedRoot $buildTargetRoot `
        -Kind 'known_rust_target' `
        -Reason '一龙历史默认路径下的可重建 Rust target'
    if ($candidate) { $candidates += $candidate }
}

if ($IncludeExpiredTemp) {
    $cutoff = [DateTime]::UtcNow.AddDays(-$MinAgeDays)
    $knownTempPrefix = '^(rustc|cargo-|elon-(aapt-inspect|remote-apk|ripgrep|codex-sharing-proof|api-runtime-env|data-root-test|data-cleanup-test))'
    foreach ($item in Get-ChildItem -LiteralPath $tempRoot -Force -ErrorAction SilentlyContinue) {
        if ($item.LastWriteTimeUtc -ge $cutoff) { continue }
        $isKnownDirectory = $item.PSIsContainer -and $item.Name -match $knownTempPrefix
        $isKnownFile = -not $item.PSIsContainer -and
            $item.Name -match $knownTempPrefix -and
            $item.Extension -in @('.tmp', '.dmp', '.dll', '.pdb')
        $isMarkedTarget = $item.PSIsContainer -and $item.Name -eq 'target' -and
            ((Test-Path -LiteralPath (Join-Path $item.FullName '.rustc_info.json')) -or
             (Test-Path -LiteralPath (Join-Path $item.FullName 'debug\.fingerprint')) -or
             (Test-Path -LiteralPath (Join-Path $item.FullName 'release\.fingerprint')))
        if (-not ($isKnownDirectory -or $isKnownFile -or $isMarkedTarget)) { continue }
        $candidate = New-Candidate `
            -Path $item.FullName `
            -AllowedRoot $tempRoot `
            -Kind 'expired_temp' `
            -Reason "严格名称/标记匹配且超过 $MinAgeDays 天"
        if ($candidate) { $candidates += $candidate }
    }
}

$candidates = @($candidates | Sort-Object Path -Unique)
if ($candidates.Count -eq 0) {
    Write-Host '没有发现符合严格规则的候选。'
    exit 0
}

$report = @()
foreach ($candidate in $candidates) {
    Assert-SafeCandidate $candidate
    $item = Get-Item -LiteralPath $candidate.Path -Force
    $report += [pscustomobject]@{
        Kind = $candidate.Kind
        GiB = [Math]::Round((Get-CandidateBytes $candidate.Path) / 1GB, 2)
        LastWriteUtc = $item.LastWriteTimeUtc.ToString('u')
        Path = $candidate.Path
        Reason = $candidate.Reason
    }
}

$report | Format-Table Kind, GiB, LastWriteUtc, Path -AutoSize

if (-not $Apply) {
    Write-Host "`nPREVIEW_ONLY=true；未删除任何内容。需要执行时显式追加 -Apply。" -ForegroundColor Yellow
    exit 0
}

$activeRust = @(Get-Process -Name cargo, rustc -ErrorAction SilentlyContinue)
if ($activeRust.Count -gt 0) {
    throw "检测到 cargo/rustc 进程，拒绝清理。PID: $($activeRust.Id -join ', ')"
}

foreach ($candidate in $candidates) {
    Assert-SafeCandidate $candidate
    if (-not $PSCmdlet.ShouldProcess($candidate.Path, '删除已知可重建缓存/过期严格 Temp 候选')) {
        continue
    }
    $item = Get-Item -LiteralPath $candidate.Path -Force
    if ($item.PSIsContainer) {
        Remove-Item -LiteralPath $candidate.Path -Recurse -Force
    } else {
        Remove-Item -LiteralPath $candidate.Path -Force
    }
    Write-Host "REMOVED=$($candidate.Path)"
}

Write-Host 'APPLY_COMPLETE=true'
