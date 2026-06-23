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
#>
param(
    [switch]$Apply
)

$ErrorActionPreference = "Stop"

$RepoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
Set-Location $RepoRoot

$manifests = @(
    "server/Cargo.toml",
    "server/pc-dev-runtime/Cargo.toml",
    "server/homecli-proto/Cargo.toml"
)

foreach ($manifest in $manifests) {
    if (-not (Test-Path $manifest)) {
        throw "Rust manifest not found: $manifest"
    }

    $manifestText = Get-Content -Raw $manifest
    if ($manifestText -notmatch '(?m)^edition\s*=\s*"[^"]+"') {
        throw "Rust manifest is missing an explicit edition: $manifest"
    }

    if ($Apply) {
        Write-Host "Formatting $manifest"
        cargo fmt --manifest-path $manifest --all
    } else {
        Write-Host "Checking $manifest"
        cargo fmt --manifest-path $manifest --all -- --check
    }
}
