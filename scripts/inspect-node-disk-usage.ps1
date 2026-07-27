#!/usr/bin/env powershell

[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [switch]$Apply,
    [switch]$IncludeActiveBuildCaches,
    [switch]$IncludeExpiredTemp,
    [switch]$IncludeReleaseHistory,
    [ValidateRange(7, 3650)]
    [int]$MinAgeDays = 30,
    [ValidateRange(1, 20)]
    [int]$ReleaseKeepNewest = 3
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0
. (Join-Path $PSScriptRoot 'node-storage-paths.ps1')

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

function Get-LatestWriteTimeUtc {
    param([Parameter(Mandatory = $true)][string]$Path)
    $latest = [DateTime]::MinValue
    $pending = New-Object 'System.Collections.Generic.Stack[string]'
    $pending.Push((Get-NormalizedFullPath $Path))
    while ($pending.Count -gt 0) {
        $current = $pending.Pop()
        $item = Get-Item -LiteralPath $current -Force
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "时间检查期间发现重解析点，拒绝继续：$current"
        }
        if ($item.LastWriteTimeUtc -gt $latest) { $latest = $item.LastWriteTimeUtc }
        if (-not $item.PSIsContainer) { continue }
        foreach ($child in Get-ChildItem -LiteralPath $current -Force -ErrorAction Stop) {
            if (($child.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "时间检查期间发现重解析点，拒绝继续：$($child.FullName)"
            }
            if ($child.LastWriteTimeUtc -gt $latest) { $latest = $child.LastWriteTimeUtc }
            if ($child.PSIsContainer) { $pending.Push($child.FullName) }
        }
    }
    return $latest
}

function New-Candidate {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$AllowedRoot,
        [Parameter(Mandatory = $true)][string]$Kind,
        [Parameter(Mandatory = $true)][string]$Reason,
        [bool]$RequiresRustIdle = $false,
        [ValidateSet('filesystem', 'git_worktree')]
        [string]$RemovalMode = 'filesystem'
    )
    if (-not (Test-Path -LiteralPath $Path)) { return $null }
    [pscustomobject]@{
        Path = Get-NormalizedFullPath $Path
        AllowedRoot = Get-NormalizedFullPath $AllowedRoot
        Kind = $Kind
        Reason = $Reason
        RequiresRustIdle = $RequiresRustIdle
        RemovalMode = $RemovalMode
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
$elonLocalRoot = Assert-SafeRoot (Join-Path $env:LOCALAPPDATA 'Elon')
$cutoff = [DateTime]::UtcNow.AddDays(-$MinAgeDays)
$tempRoot = $null
if ($IncludeExpiredTemp) {
    $tempRoot = Assert-SafeRoot ([System.IO.Path]::GetTempPath())
}

$candidates = @()
$legacyDev = New-Candidate `
    -Path (Join-Path $buildTargetRoot 'elon-dev-cargo') `
    -AllowedRoot $buildTargetRoot `
    -Kind 'retired_dev_target' `
    -Reason '新版 cargo-dev 已改用 rust-cache-v2；这是旧版共享开发 target' `
    -RequiresRustIdle $true
if ($legacyDev) { $candidates += $legacyDev }

foreach ($item in Get-ChildItem -Directory -LiteralPath $buildTargetRoot -Filter 'elon-build-*' -ErrorAction SilentlyContinue) {
    $hasRustTargetMarker =
        (Test-Path -LiteralPath (Join-Path $item.FullName '.rustc_info.json') -PathType Leaf) -or
        (Test-Path -LiteralPath (Join-Path $item.FullName 'debug\.fingerprint') -PathType Container) -or
        (Test-Path -LiteralPath (Join-Path $item.FullName 'release\.fingerprint') -PathType Container)
    if (-not $hasRustTargetMarker) { continue }
    $candidate = New-Candidate `
        -Path $item.FullName `
        -AllowedRoot $buildTargetRoot `
        -Kind 'orphaned_build_target' `
        -Reason "严格匹配 elon-build-* 且带 Rust target 标记、整棵目录超过 $MinAgeDays 天" `
        -RequiresRustIdle $true
    if ($candidate) { $candidates += $candidate }
}

$activeRustRoot = $null
if (-not [string]::IsNullOrWhiteSpace($env:ELON_RUST_CACHE_ROOT)) {
    $activeRustRoot = Get-NormalizedFullPath $env:ELON_RUST_CACHE_ROOT
} elseif ($nodeDataRoot = Get-ElonNodeDataRoot) {
    $activeRustRoot = Get-NormalizedFullPath (Join-Path $nodeDataRoot 'cache\rust-cache-v2')
}
$localRustV2 = Join-Path $elonLocalRoot 'rust-cache-v2'
if (
    (Test-Path -LiteralPath $localRustV2 -PathType Container) -and
    $activeRustRoot -and
    (Get-NormalizedFullPath $localRustV2) -ne $activeRustRoot
) {
    $candidate = New-Candidate `
        -Path $localRustV2 `
        -AllowedRoot $elonLocalRoot `
        -Kind 'retired_rust_cache_v2' `
        -Reason "当前 rust-cache-v2 已解析到 $activeRustRoot；C 盘旧根不再是写入位置" `
        -RequiresRustIdle $true
    if ($candidate) { $candidates += $candidate }
}

if ($IncludeActiveBuildCaches) {
    foreach ($entry in @(
        @{
            Name = 'elon-node-agent'
            Reason = '当前节点发布最终 target；删除后下一次发布会重新链接/编译'
        },
        @{
            Name = 'elon-server-musl'
            Reason = '旧默认服务器 musl 发布 target；删除后下一次发布可能全量重编'
        }
    )) {
        $candidate = New-Candidate `
            -Path (Join-Path $buildTargetRoot $entry.Name) `
            -AllowedRoot $buildTargetRoot `
            -Kind 'active_rebuildable_target' `
            -Reason $entry.Reason `
            -RequiresRustIdle $true
        if ($candidate) { $candidates += $candidate }
    }
}

if ($IncludeExpiredTemp) {
    $knownTempPrefix = '^(rustc|cargo-|elon-|ElonSpeed$|elonspeed-|cofficethinking[-_]|bb64a-|codex-.*-chrome|fb2-.*-chrome|HeadlessChrome|chromiumoxide-runner$|gradle-extract$|node-compile-cache$|vscode-safe$|vscode-stable-user-|WinGet$|Roslyn$|Diagnostics$|DiagOutputDir$)'
    foreach ($item in Get-ChildItem -LiteralPath $tempRoot -Force -ErrorAction SilentlyContinue) {
        if ($item.LastWriteTimeUtc -ge $cutoff) { continue }
        $isKnownDirectory = $item.PSIsContainer -and $item.Name -match $knownTempPrefix
        $isKnownFile = -not $item.PSIsContainer -and
            $item.Extension -in @('.tmp', '.dmp', '.log', '.etl', '.pdb')
        $isMarkedTarget = $item.PSIsContainer -and $item.Name -eq 'target' -and
            ((Test-Path -LiteralPath (Join-Path $item.FullName '.rustc_info.json')) -or
             (Test-Path -LiteralPath (Join-Path $item.FullName 'debug\.fingerprint')) -or
             (Test-Path -LiteralPath (Join-Path $item.FullName 'release\.fingerprint')))
        if (-not ($isKnownDirectory -or $isKnownFile -or $isMarkedTarget)) { continue }
        $candidate = New-Candidate `
            -Path $item.FullName `
            -AllowedRoot $tempRoot `
            -Kind 'expired_temp' `
            -Reason "严格名称、文件类型或 Rust target 标记匹配且整棵目录超过 $MinAgeDays 天" `
            -RequiresRustIdle ($item.Name -match '^(rustc|cargo-|elon-cargo)')
        if ($candidate) { $candidates += $candidate }
    }
}

if ($IncludeReleaseHistory) {
    $outboxRoot = Join-Path $elonLocalRoot 'release-outbox-v1'
    $eventsRoot = Join-Path $outboxRoot 'events'
    $sourcesRoot = Join-Path $outboxRoot 'sources'
    if (Test-Path -LiteralPath $eventsRoot -PathType Container) {
        foreach ($eventFile in Get-ChildItem -File -Recurse -Filter 'event.json' -LiteralPath $eventsRoot -ErrorAction SilentlyContinue) {
            try {
                $event = Read-ElonStorageJson -Path $eventFile.FullName
                if ([string]$event.sync_state -notin @('synced', 'failed')) { continue }
                $eventDir = $eventFile.Directory.FullName
                $eventCandidate = New-Candidate `
                    -Path $eventDir `
                    -AllowedRoot $outboxRoot `
                    -Kind 'terminal_outbox_event' `
                    -Reason "发布 outbox 已终止（$([string]$event.sync_state)）且整棵目录超过 $MinAgeDays 天"
                if ($eventCandidate) { $candidates += $eventCandidate }

                $sha = $eventFile.Directory.Name
                $sourceDir = Join-Path $sourcesRoot $sha
                $sourceCandidate = New-Candidate `
                    -Path $sourceDir `
                    -AllowedRoot $outboxRoot `
                    -Kind 'terminal_outbox_source' `
                    -Reason '对应发布 outbox 已终止；通过 Git worktree remove 精确回收源码快照' `
                    -RemovalMode 'git_worktree'
                if ($sourceCandidate) { $candidates += $sourceCandidate }
            } catch {
                Write-Warning "跳过无法解析的 outbox 事件：$($eventFile.FullName)"
            }
        }
    }

    $activationRoot = Join-Path $elonLocalRoot 'local-node-releases-v1'
    $releasesRoot = Join-Path $activationRoot 'releases'
    if (Test-Path -LiteralPath $releasesRoot -PathType Container) {
        $states = @(
            Get-ChildItem -File -Recurse -Filter 'state.json' -LiteralPath $releasesRoot -ErrorAction SilentlyContinue |
                ForEach-Object {
                    try {
                        $state = Read-ElonStorageJson -Path $_.FullName
                        [pscustomobject]@{
                            File = $_
                            State = $state
                            VerifiedAtMs = [long]$state.verified_at_ms
                        }
                    } catch {
                        Write-Warning "跳过无法解析的本地发布状态：$($_.FullName)"
                    }
                } |
                Sort-Object VerifiedAtMs -Descending
        )
        foreach ($entry in @($states | Select-Object -Skip $ReleaseKeepNewest)) {
            $activationState = [string]$entry.State.activation_state
            $terminalState = [string]$entry.State.local_terminal_state
            if ($activationState -notin @('superseded', 'failed') -or
                $terminalState -notin @('complete', 'failed')) {
                continue
            }
            $sha = $entry.File.Directory.Name
            foreach ($path in @(
                $entry.File.Directory.FullName,
                (Join-Path $activationRoot (Join-Path 'apply' $sha))
            )) {
                $candidate = New-Candidate `
                    -Path $path `
                    -AllowedRoot $activationRoot `
                    -Kind 'terminal_local_release' `
                    -Reason "本地发布已终止（$activationState/$terminalState）、不在最新 $ReleaseKeepNewest 份保护范围且超过 $MinAgeDays 天"
                if ($candidate) { $candidates += $candidate }
            }
        }
    }
}

$candidates = @($candidates | Sort-Object Path -Unique)
$verifiedCandidates = @()
foreach ($candidate in $candidates) {
    Assert-SafeCandidate $candidate
    $latestWriteUtc = Get-LatestWriteTimeUtc $candidate.Path
    if ($candidate.Kind -notin @('active_rebuildable_target', 'retired_dev_target') -and
        $latestWriteUtc -ge $cutoff) {
        continue
    }
    $candidate | Add-Member -NotePropertyName LatestWriteUtc -NotePropertyValue $latestWriteUtc
    $verifiedCandidates += $candidate
}
$candidates = @($verifiedCandidates)
if ($candidates.Count -eq 0) {
    Write-Host '没有发现符合严格规则的候选。'
    return
}

$report = @()
foreach ($candidate in $candidates) {
    $estimatedBytes = Get-CandidateBytes $candidate.Path
    $candidate | Add-Member -NotePropertyName EstimatedBytes -NotePropertyValue $estimatedBytes
    $report += [pscustomobject]@{
        Kind = $candidate.Kind
        GiB = [Math]::Round($estimatedBytes / 1GB, 2)
        LastWriteUtc = $candidate.LatestWriteUtc.ToString('u')
        Path = $candidate.Path
        Reason = $candidate.Reason
    }
}

$report | Format-Table Kind, GiB, LastWriteUtc, Path -AutoSize

if (-not $Apply) {
    Write-Host "`nPREVIEW_ONLY=true；未删除任何内容。需要执行时显式追加 -Apply。" -ForegroundColor Yellow
    return
}

$activeRust = @(Get-Process -Name cargo, rustc -ErrorAction SilentlyContinue)
$skippedRust = @()
$removedBytes = [uint64]0
$removedCount = 0

foreach ($candidate in $candidates) {
    if ($candidate.RequiresRustIdle -and $activeRust.Count -gt 0) {
        $skippedRust += $candidate.Path
        continue
    }
    Assert-SafeCandidate $candidate
    if (-not $PSCmdlet.ShouldProcess($candidate.Path, '删除已知可重建缓存/过期严格 Temp 候选')) {
        continue
    }
    if ($candidate.RemovalMode -eq 'git_worktree') {
        $status = @(& git -C $candidate.Path status --porcelain 2>&1)
        if ($LASTEXITCODE -ne 0 -or -not [string]::IsNullOrWhiteSpace(($status -join "`n"))) {
            Write-Warning "Git worktree 不可验证或不干净，已跳过：$($candidate.Path)"
            continue
        }
        & git -C $PSScriptRoot worktree remove --force $candidate.Path
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "Git worktree remove 失败，已保留：$($candidate.Path)"
            continue
        }
    } else {
        $item = Get-Item -LiteralPath $candidate.Path -Force
        if ($item.PSIsContainer) {
            Remove-Item -LiteralPath $candidate.Path -Recurse -Force
        } else {
            Remove-Item -LiteralPath $candidate.Path -Force
        }
    }
    $removedBytes += [uint64]$candidate.EstimatedBytes
    $removedCount += 1
    Write-Host "REMOVED=$($candidate.Path)"
}

if ($skippedRust.Count -gt 0) {
    Write-Warning "检测到 cargo/rustc，已跳过 $($skippedRust.Count) 个 Rust 候选。PID: $($activeRust.Id -join ', ')"
}
Write-Host "REMOVED_COUNT=$removedCount"
Write-Host "RECLAIMED_GIB=$([Math]::Round($removedBytes / 1GB, 2))"
Write-Host 'APPLY_COMPLETE=true'
