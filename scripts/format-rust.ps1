<#
.SYNOPSIS
    Format or check Rust crates using each crate manifest edition.

.DESCRIPTION
    This script intentionally avoids bare `rustfmt` and bare `cargo fmt`.
    Each cargo invocation points at a concrete Cargo.toml so rustfmt receives
    the crate edition from the manifest.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\format-rust.ps1

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\format-rust.ps1 -Apply

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\format-rust.ps1 -Apply -Files server/src/main.rs
#>
param(
    [switch]$Apply,
    [string[]]$Files = @(),
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$RemainingFiles = @()
)

$ErrorActionPreference = "Stop"

$RepoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
Set-Location $RepoRoot

$crates = @(
    @{ Root = "server"; Manifest = "server/Cargo.toml" },
    @{ Root = "server/pc-dev-runtime"; Manifest = "server/pc-dev-runtime/Cargo.toml" },
    @{ Root = "server/homecli-proto"; Manifest = "server/homecli-proto/Cargo.toml" }
)

function Get-CrateEdition {
    param([string]$Manifest)

    $manifestText = Get-Content -Raw $Manifest
    if ($manifestText -notmatch '(?m)^edition\s*=\s*"([^"]+)"') {
        throw "Rust manifest is missing an explicit edition: $Manifest"
    }
    return $Matches[1]
}

function Invoke-NativeCommand {
    param(
        [string]$Command,
        [string[]]$Arguments
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

function Convert-ToRepoRelativePath {
    param([string]$Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        $fullPath = [System.IO.Path]::GetFullPath($Path)
    } else {
        $fullPath = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $Path))
    }

    $repoFullPath = [System.IO.Path]::GetFullPath($RepoRoot).TrimEnd('\', '/')
    $comparison = [System.StringComparison]::OrdinalIgnoreCase
    if (-not ($fullPath.Equals($repoFullPath, $comparison) -or $fullPath.StartsWith("$repoFullPath\", $comparison) -or $fullPath.StartsWith("$repoFullPath/", $comparison))) {
        throw "Rust file is outside repository: $Path"
    }

    $repoUri = [System.Uri]::new(($repoFullPath.Replace('\', '/') + '/'))
    $fileUri = [System.Uri]::new($fullPath.Replace('\', '/'))
    return [System.Uri]::UnescapeDataString($repoUri.MakeRelativeUri($fileUri).ToString())
}

foreach ($crate in $crates) {
    $manifest = $crate["Manifest"]
    if (-not (Test-Path $manifest)) {
        throw "Rust manifest not found: $manifest"
    }
    $crate["Edition"] = Get-CrateEdition $manifest
}

$requestedFiles = @($Files) + @($RemainingFiles)

if ($requestedFiles.Count -gt 0) {
    $groups = @{}
    foreach ($file in ($requestedFiles | Select-Object -Unique)) {
        $relative = Convert-ToRepoRelativePath $file
        if ($relative -notmatch '\.rs$') {
            continue
        }
        if (-not (Test-Path $relative)) {
            throw "Rust file not found: $relative"
        }

        $crate = $crates |
            Sort-Object { $_["Root"].Length } -Descending |
            Where-Object { $relative -eq $_["Root"] -or $relative.StartsWith("$($_["Root"])/", [System.StringComparison]::Ordinal) } |
            Select-Object -First 1
        if (-not $crate) {
            throw "Rust file is not under a known crate: $relative"
        }

        $edition = $crate["Edition"]
        if (-not $groups.ContainsKey($edition)) {
            $groups[$edition] = New-Object System.Collections.Generic.List[string]
        }
        $groups[$edition].Add($relative)
    }

    if ($groups.Count -eq 0) {
        Write-Host "No Rust files to format"
        exit 0
    }

    foreach ($edition in $groups.Keys) {
        $rustfmtArgs = @("--edition", $edition, "--config", "skip_children=true")
        if (-not $Apply) {
            $rustfmtArgs += "--check"
        }
        $rustfmtArgs += @($groups[$edition])
        if ($Apply) {
            Write-Host "Formatting $($groups[$edition].Count) Rust file(s) with edition $edition"
        } else {
            Write-Host "Checking $($groups[$edition].Count) Rust file(s) with edition $edition"
        }
        Invoke-NativeCommand "rustfmt" $rustfmtArgs
    }
    exit 0
}

foreach ($crate in $crates) {
    if ($Apply) {
        Write-Host "Formatting $($crate["Manifest"])"
        Invoke-NativeCommand "cargo" @("fmt", "--manifest-path", $crate["Manifest"], "--all")
    } else {
        Write-Host "Checking $($crate["Manifest"])"
        Invoke-NativeCommand "cargo" @("fmt", "--manifest-path", $crate["Manifest"], "--all", "--", "--check")
    }
}
