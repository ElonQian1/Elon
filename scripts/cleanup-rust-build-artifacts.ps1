<#
.SYNOPSIS
    Preview or remove local Rust build artifacts left by Elon worktrees.

.DESCRIPTION
    Cargo target directories are fully reproducible, but isolated validation
    runs can leave large target trees behind after a worktree is no longer
    useful. This script only accepts targets discovered from registered Elon
    worktrees, a verified orphan task directory, or an explicitly supplied
    Cargo target directory. It never removes source, Git metadata, or data.

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cleanup-rust-build-artifacts.ps1

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cleanup-rust-build-artifacts.ps1 -Apply

.EXAMPLE
    & .\scripts\cleanup-rust-build-artifacts.ps1 -Apply -AdditionalTarget @(
        'D:\old-release-target', 'D:\old-cross-build-target'
    )
#>
[CmdletBinding()]
param(
    [switch]$Apply,
    [string]$RepoRoot = "",
    [string]$WorkspaceRoot = "",
    [string[]]$AdditionalTarget = @()
)

$ErrorActionPreference = "Stop"

function Resolve-FullPath {
    param([Parameter(Mandatory)][string]$Path)

    if (-not [System.IO.Path]::IsPathRooted($Path)) {
        throw "Path must be absolute: $Path"
    }
    return [System.IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
}

function Get-DirectorySize {
    param([Parameter(Mandatory)][string]$Path)

    [int64]$sum = 0
    foreach ($file in Get-ChildItem -LiteralPath $Path -File -Force -Recurse -ErrorAction SilentlyContinue) {
        $sum += [int64]$file.Length
    }
    return $sum
}

function Test-CargoTargetSignature {
    param([Parameter(Mandatory)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        return $false
    }
    if ((Test-Path -LiteralPath (Join-Path $Path ".rustc_info.json")) -or
        (Test-Path -LiteralPath (Join-Path $Path "CACHEDIR.TAG"))) {
        return $true
    }
    $names = @(Get-ChildItem -LiteralPath $Path -Directory -Force -ErrorAction SilentlyContinue | ForEach-Object Name)
    return ($names -contains "debug") -or ($names -contains "release") -or
        (@($names | Where-Object { $_ -match '^rust-(test|check)-' }).Count -gt 0)
}

function Get-GitWorktreePaths {
    param([Parameter(Mandatory)][string]$Root)

    $paths = New-Object System.Collections.Generic.List[string]
    $output = & git -C $Root worktree list --porcelain 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to enumerate Git worktrees: $($output -join "`n")"
    }
    foreach ($line in $output) {
        if ($line -like "worktree *") {
            $path = $line.Substring("worktree ".Length)
            if (Test-Path -LiteralPath $path -PathType Container) {
                $paths.Add((Resolve-FullPath -Path $path))
            }
        }
    }
    return @($paths | Select-Object -Unique)
}

function Get-RepositoryLeaf {
    param([Parameter(Mandatory)][string]$Root)

    $primary = Get-GitWorktreePaths -Root $Root | Where-Object {
        Test-Path -LiteralPath (Join-Path $_ ".git") -PathType Container
    } | Select-Object -First 1
    if ($primary) {
        return Split-Path -Leaf $primary
    }
    return Split-Path -Leaf $Root
}

function Add-TargetCandidate {
    param(
        [Parameter(Mandatory)][hashtable]$Candidates,
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Source
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        return
    }
    $fullPath = Resolve-FullPath -Path $Path
    $key = $fullPath.ToLowerInvariant()
    if (-not $Candidates.ContainsKey($key)) {
        $Candidates[$key] = [pscustomobject]@{
            path = $fullPath
            sources = New-Object System.Collections.Generic.List[string]
        }
    }
    $Candidates[$key].sources.Add($Source)
}

function Get-TargetCandidates {
    param(
        [Parameter(Mandatory)][string]$Root,
        [Parameter(Mandatory)][string]$Parent,
        [string[]]$ExtraTargets = @()
    )

    $candidates = @{}
    foreach ($worktree in Get-GitWorktreePaths -Root $Root) {
        foreach ($relative in @("target", "server\target")) {
            $target = Join-Path $worktree $relative
            if ((Test-Path -LiteralPath $target -PathType Container) -and (Test-CargoTargetSignature -Path $target)) {
                Add-TargetCandidate -Candidates $candidates -Path $target -Source "registered-worktree:$worktree"
            }
        }
    }

    $repoLeaf = Get-RepositoryLeaf -Root $Root
    $taskPattern = "^$([regex]::Escape($repoLeaf))-task-"
    foreach ($directory in Get-ChildItem -LiteralPath $Parent -Directory -Force -ErrorAction SilentlyContinue) {
        if ($directory.Name -notmatch $taskPattern -or (Test-Path -LiteralPath (Join-Path $directory.FullName ".git"))) {
            continue
        }
        $children = @(Get-ChildItem -LiteralPath $directory.FullName -Force -ErrorAction SilentlyContinue)
        $target = Join-Path $directory.FullName "target"
        if ($children.Count -eq 1 -and $children[0].Name -eq "target" -and (Test-CargoTargetSignature -Path $target)) {
            Add-TargetCandidate -Candidates $candidates -Path $target -Source "orphan-task-directory:$($directory.FullName)"
        }
    }

    foreach ($target in $ExtraTargets) {
        $fullPath = Resolve-FullPath -Path $target
        if (-not (Test-CargoTargetSignature -Path $fullPath)) {
            throw "AdditionalTarget is not a recognized Cargo target directory: $fullPath"
        }
        Add-TargetCandidate -Candidates $candidates -Path $fullPath -Source "explicit-additional-target"
    }
    return @($candidates.Values | Sort-Object path)
}

function Remove-TargetDirectory {
    param([Parameter(Mandatory)][string]$Path)

    $fullPath = Resolve-FullPath -Path $Path
    try {
        Remove-Item -LiteralPath $fullPath -Force -Recurse -ErrorAction Stop
    } catch {
        $longPath = if ($fullPath.StartsWith("\\?\")) { $fullPath } else { "\\?\$fullPath" }
        [System.IO.Directory]::Delete($longPath, $true)
    }
}

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (& git rev-parse --show-toplevel 2>$null).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($RepoRoot)) {
        throw "RepoRoot was not supplied and the current directory is not a Git worktree."
    }
}
$RepoRoot = Resolve-FullPath -Path $RepoRoot
if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot ".git"))) {
    throw "RepoRoot is not a Git worktree: $RepoRoot"
}
if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    $WorkspaceRoot = Split-Path -Parent $RepoRoot
}
$WorkspaceRoot = Resolve-FullPath -Path $WorkspaceRoot
if (-not (Test-Path -LiteralPath $WorkspaceRoot -PathType Container)) {
    throw "WorkspaceRoot does not exist: $WorkspaceRoot"
}

$activeBuilds = @(Get-Process -Name cargo, rustc -ErrorAction SilentlyContinue)
if ($Apply -and $activeBuilds.Count -gt 0) {
    $processes = ($activeBuilds | ForEach-Object { "$($_.ProcessName):$($_.Id)" }) -join ", "
    throw "Refusing cleanup while Cargo or rustc is active: $processes"
}

$targets = @(Get-TargetCandidates -Root $RepoRoot -Parent $WorkspaceRoot -ExtraTargets $AdditionalTarget)
$mode = if ($Apply) { "apply" } else { "preview" }
Write-Output "RUST_TARGET_CLEANUP_MODE=$mode"
Write-Output "RUST_TARGET_CLEANUP_CANDIDATE_COUNT=$($targets.Count)"

[int64]$reclaimableBytes = 0
foreach ($target in $targets) {
    if (-not (Test-CargoTargetSignature -Path $target.path)) {
        throw "Target signature changed during cleanup scan: $($target.path)"
    }
    $size = Get-DirectorySize -Path $target.path
    $reclaimableBytes += $size
    Write-Output "RUST_TARGET_CLEANUP_CANDIDATE=$($target.path)"
    Write-Output "RUST_TARGET_CLEANUP_SOURCE=$($target.sources -join ',')"
    Write-Output "RUST_TARGET_CLEANUP_BYTES=$size"
}

if (-not $Apply) {
    Write-Output "RUST_TARGET_CLEANUP_RECLAIMABLE_BYTES=$reclaimableBytes"
    Write-Output "RUST_TARGET_CLEANUP_RESULT=preview"
    return
}

$removed = 0
foreach ($target in $targets) {
    Remove-TargetDirectory -Path $target.path
    if (Test-Path -LiteralPath $target.path) {
        throw "Target directory still exists after cleanup: $($target.path)"
    }
    $removed++
    Write-Output "RUST_TARGET_CLEANUP_REMOVED=$($target.path)"
}
Write-Output "RUST_TARGET_CLEANUP_RECLAIMED_BYTES=$reclaimableBytes"
Write-Output "RUST_TARGET_CLEANUP_REMOVED_COUNT=$removed"
Write-Output "RUST_TARGET_CLEANUP_RESULT=applied"
