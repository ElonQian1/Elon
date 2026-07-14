param()

$ErrorActionPreference = "Stop"

function Invoke-Captured {
    param(
        [string]$Command,
        [string[]]$Arguments
    )

    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $Command @Arguments 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    return [pscustomobject]@{
        ExitCode = $exitCode
        Text = (($output | ForEach-Object { [string]$_ }) -join "`n").Trim()
    }
}

function Assert-Contains {
    param([string]$Text, [string]$Expected, [string]$Message)
    if (-not $Text.Contains($Expected)) {
        throw "$Message Missing: $Expected`nActual:`n$Text"
    }
}

function Resolve-BashCommand {
    $available = Get-Command bash -ErrorAction SilentlyContinue
    if ($available) {
        return $available.Source
    }

    $gitCommand = Get-Command git -ErrorAction Stop
    $gitRoot = Split-Path (Split-Path $gitCommand.Source -Parent) -Parent
    foreach ($candidate in @(
        (Join-Path $gitRoot "bin\bash.exe"),
        (Join-Path $gitRoot "usr\bin\bash.exe")
    )) {
        if (Test-Path -LiteralPath $candidate) {
            return $candidate
        }
    }
    throw "bash is required to verify scripts/format-rust.sh."
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$psScript = Join-Path $repoRoot "scripts\format-rust.ps1"
$shScript = Join-Path $repoRoot "scripts\format-rust.sh"
$bashCommand = Resolve-BashCommand

$psRefusal = Invoke-Captured -Command "powershell" -Arguments @(
    "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $psScript, "-Apply"
)
if ($psRefusal.ExitCode -ne 2) {
    throw "PowerShell formatter must reject implicit full writes with exit 2.`n$($psRefusal.Text)"
}
Assert-Contains $psRefusal.Text "-Apply -All" "PowerShell refusal must show the explicit full-write command."

$shRefusal = Invoke-Captured -Command $bashCommand -Arguments @(($shScript -replace '\\', '/'), "--apply")
if ($shRefusal.ExitCode -ne 2) {
    throw "Shell formatter must reject implicit full writes with exit 2.`n$($shRefusal.Text)"
}
Assert-Contains $shRefusal.Text "--apply --all" "Shell refusal must show the explicit full-write command."

$psContent = Get-Content -Raw -LiteralPath $psScript -Encoding UTF8
$shContent = Get-Content -Raw -LiteralPath $shScript -Encoding UTF8
Assert-Contains $psContent "Test-FullFormatClean" "PowerShell full apply must verify convergence."
Assert-Contains $psContent 'for ($pass = 1; $pass -le 3; $pass++)' "PowerShell full apply must retry until idempotent."
Assert-Contains $psContent "status --porcelain=v1 --untracked-files=all" "PowerShell full apply must require a clean worktree."
Assert-Contains $shContent "full_format_clean" "Shell full apply must verify convergence."
Assert-Contains $shContent "for pass in 1 2 3" "Shell full apply must retry until idempotent."
Assert-Contains $shContent "git status --porcelain=v1 --untracked-files=all" "Shell full apply must require a clean worktree."

$fullCheck = Invoke-Captured -Command "powershell" -Arguments @(
    "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $psScript
)
if ($fullCheck.ExitCode -ne 0) {
    throw "Repository-wide Rust format baseline is not clean.`n$($fullCheck.Text)"
}

$workflowDoc = Get-Content -Raw -LiteralPath (Join-Path $repoRoot ".github\instructions\git-deploy-workflow.instructions.md") -Encoding UTF8
Assert-Contains $workflowDoc "-Apply -All" "Workflow documentation must describe explicit full formatting."
Assert-Contains $workflowDoc "style(rust)" "Workflow documentation must require a separate mechanical-format commit."

$hook = Get-Content -Raw -LiteralPath (Join-Path $repoRoot ".githooks\pre-push") -Encoding UTF8
Assert-Contains $hook "bash scripts/format-rust.sh" "Pre-push must enforce the full Rust format baseline."
Assert-Contains $hook 'unset $git_local_env_vars' "Pre-push must clear shared Git variables before checking a linked worktree."

$toolchain = Get-Content -Raw -LiteralPath (Join-Path $repoRoot "rust-toolchain.toml") -Encoding UTF8
Assert-Contains $toolchain 'channel = "stable"' "Rust toolchain must declare the stable baseline channel without forcing a network-only exact toolchain install."
Assert-Contains $toolchain 'components = ["rustfmt"]' "Pinned toolchain must install rustfmt."

$versionLock = Get-Content -Raw -LiteralPath (Join-Path $repoRoot ".rustfmt-version") -Encoding UTF8
Assert-Contains $versionLock 'rustfmt 1.9.0-stable (31fca3adb2 2026-06-26)' "Rustfmt build must stay locked to the committed baseline."

Write-Host "PASS Rust format workflow guard"
