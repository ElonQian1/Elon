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
    powershell -ExecutionPolicy Bypass -File scripts\format-rust.ps1 -Apply -All

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\format-rust.ps1 -Apply -Files server/src/main.rs
#>
param(
    [switch]$Apply,
    [switch]$All,
    [string[]]$Files = @(),
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$RemainingFiles = @()
)

$ErrorActionPreference = "Stop"

$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
Set-Location $RepoRoot

$rustfmtVersionFile = Join-Path $RepoRoot ".rustfmt-version"
if (-not (Test-Path -LiteralPath $rustfmtVersionFile)) {
    throw "Rust formatter version lock is missing: .rustfmt-version"
}
$expectedRustfmtVersion = (Get-Content -Raw -LiteralPath $rustfmtVersionFile).Trim()
$actualRustfmtVersion = (& rustfmt --version 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "rustfmt is unavailable. Install the rustfmt component for the repository toolchain."
}
if ($actualRustfmtVersion -ne $expectedRustfmtVersion) {
    throw "rustfmt version mismatch. Expected '$expectedRustfmtVersion', got '$actualRustfmtVersion'. Use the baseline toolchain or create a dedicated format-baseline migration."
}

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

if ($All -and $requestedFiles.Count -gt 0) {
    [Console]::Error.WriteLine("Choose either -All or -Files; they cannot be combined.")
    exit 2
}

if ($Apply -and -not $All -and $requestedFiles.Count -eq 0) {
    [Console]::Error.WriteLine("Refusing an implicit repository-wide write. Use -Apply -Files <changed.rs...> for daily work, or -Apply -All in a dedicated format-only task.")
    exit 2
}

if ($Apply -and $All) {
    $worktreeStatus = (& git -C $RepoRoot status --porcelain=v1 --untracked-files=all 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to inspect the worktree before full formatting: $worktreeStatus"
    }
    if (-not [string]::IsNullOrWhiteSpace($worktreeStatus)) {
        [Console]::Error.WriteLine("Refusing a repository-wide write in a dirty worktree. Commit or isolate existing changes, then run -Apply -All from a clean dedicated task.")
        exit 2
    }
}

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

if (-not $Apply) {
    foreach ($crate in $crates) {
        Write-Host "Checking $($crate["Manifest"])"
        Invoke-NativeCommand "cargo" @("fmt", "--manifest-path", $crate["Manifest"], "--all", "--", "--check")
    }
    exit 0
}

function Test-FullFormatClean {
    foreach ($crate in $crates) {
        $manifest = $crate["Manifest"]
        $oldPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            & cargo fmt --manifest-path $manifest --all -- --check *> $null
            $exitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $oldPreference
        }
        if ($exitCode -ne 0) {
            return $false
        }
    }
    return $true
}

$converged = $false
for ($pass = 1; $pass -le 3; $pass++) {
    foreach ($crate in $crates) {
        Write-Host "Formatting $($crate["Manifest"]) (pass $pass/3)"
        Invoke-NativeCommand "cargo" @("fmt", "--manifest-path", $crate["Manifest"], "--all")
    }
    if (Test-FullFormatClean) {
        Write-Host "Full Rust format converged after $pass pass(es)"
        $converged = $true
        break
    }
}

if (-not $converged) {
    [Console]::Error.WriteLine("Full Rust format did not converge after 3 passes. Running a visible check for diagnostics.")
    foreach ($crate in $crates) {
        Invoke-NativeCommand "cargo" @("fmt", "--manifest-path", $crate["Manifest"], "--all", "--", "--check")
    }
    exit 1
}
