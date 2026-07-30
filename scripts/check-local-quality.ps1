<#
.SYNOPSIS
    Run the local quality gate set from one stable entry point.

.DESCRIPTION
    Static mode is the fast default for daily work. Server, Frontend, and All
    expand the check surface before release or risky refactors.
#>
param(
    [ValidateSet("Static", "Server", "Frontend", "All")]
    [string]$Scope = "Static",
    [switch]$SkipDependencyAudit,
    [switch]$SkipRustWarningBudget,
    [switch]$SkipCargoTest,
    [switch]$SkipFrontendInstall
)

$ErrorActionPreference = "Stop"

function Stop-LocalQualityGate {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Get-RepoRoot {
    $root = (& git rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($root)) {
        Stop-LocalQualityGate "Current directory is not inside a git repository."
    }
    return $root
}

function Invoke-CheckedCommand {
    param(
        [string]$Name,
        [string]$FilePath,
        [string[]]$Arguments = @(),
        [string]$WorkingDirectory = ""
    )

    Write-Host "== $Name =="
    $originalLocation = Get-Location
    try {
        if (-not [string]::IsNullOrWhiteSpace($WorkingDirectory)) {
            Set-Location $WorkingDirectory
        }
        & $FilePath @Arguments
        $exitCode = $LASTEXITCODE
    } finally {
        Set-Location $originalLocation
    }

    if ($null -eq $exitCode) {
        $exitCode = 0
    }
    if ($exitCode -ne 0) {
        Stop-LocalQualityGate "$Name failed with exit code $exitCode."
    }
}

function Invoke-RepoPowerShellScript {
    param(
        [string]$Name,
        [string]$ScriptPath,
        [string[]]$Arguments = @()
    )

    $fullScriptPath = Join-Path $repoRoot $ScriptPath
    $psArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $fullScriptPath) + $Arguments
    Invoke-CheckedCommand -Name $Name -FilePath "powershell" -Arguments $psArgs -WorkingDirectory $repoRoot
}

function Invoke-NpmCommand {
    param(
        [string]$Name,
        [string[]]$Arguments
    )

    $npmCommand = @(Get-Command npm.cmd -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1)
    $npmPath = if ($npmCommand.Count -gt 0 -and -not [string]::IsNullOrWhiteSpace([string]$npmCommand[0].Source)) {
        [string]$npmCommand[0].Source
    } else {
        "npm"
    }

    Invoke-CheckedCommand -Name $Name -FilePath $npmPath -Arguments $Arguments -WorkingDirectory (Join-Path $repoRoot "pc-frontend")
}

$repoRoot = Get-RepoRoot
Set-Location $repoRoot

$runsServer = $Scope -eq "Server" -or $Scope -eq "All"
$runsFrontend = $Scope -eq "Frontend" -or $Scope -eq "All"
$snapshotArgs = @()
if ($Scope -eq "Static" -or $SkipCargoTest) {
    $snapshotArgs += "-SkipCargoTest"
}

Invoke-RepoPowerShellScript -Name "Source Size Guard" -ScriptPath "scripts\check-source-size.ps1"
Invoke-RepoPowerShellScript -Name "Document Modularity Guard" -ScriptPath "scripts\check-document-modularity.ps1"
Invoke-RepoPowerShellScript -Name "Release Runbook Guard" -ScriptPath "scripts\check-release-runbook.ps1"
Invoke-RepoPowerShellScript -Name "CI Quality Gates Guard" -ScriptPath "scripts\check-ci-quality-gates.ps1"
Invoke-RepoPowerShellScript -Name "APP UI Fast Lane Guard" -ScriptPath "scripts\test-app-ui-fast-lane.ps1"
Invoke-RepoPowerShellScript -Name "Realtime Runbook Guard" -ScriptPath "scripts\check-realtime-runbook.ps1"
Invoke-RepoPowerShellScript -Name "Realtime Ownership Guard" -ScriptPath "scripts\check-realtime-ownership.ps1"
Invoke-RepoPowerShellScript -Name "Realtime Diagnostics Snapshot Guard" -ScriptPath "scripts\check-realtime-diagnostics-snapshot.ps1" -Arguments $snapshotArgs

if ($runsServer) {
    if ([string]::IsNullOrWhiteSpace($env:RUST_TEST_THREADS)) {
        $env:RUST_TEST_THREADS = "1"
    }

    if (-not $SkipDependencyAudit) {
        Invoke-RepoPowerShellScript -Name "Rust Dependency Audit" -ScriptPath "scripts\check-dependency-audit.ps1" -Arguments @(
            "-Mode",
            "Strict",
            "-SkipNpm",
            "-RequireRustAudit",
            "-AllowStaleRustAdvisoryDb"
        )
    }

    if (-not $SkipRustWarningBudget) {
        Invoke-RepoPowerShellScript -Name "Rust Warning Budget" -ScriptPath "scripts\check-rust-warning-budget.ps1" -Arguments @("-MaxWarnings", "0")
    }

    if (-not $SkipCargoTest) {
        Invoke-RepoPowerShellScript -Name "Cargo Test" -ScriptPath "scripts\cargo-dev.ps1" -Arguments @(
            "test",
            "--manifest-path",
            "server\Cargo.toml"
        )
    }
}

if ($runsFrontend) {
    if (-not $SkipFrontendInstall) {
        Invoke-NpmCommand -Name "PC Frontend Install" -Arguments @("ci")
    }

    if (-not $SkipDependencyAudit) {
        Invoke-RepoPowerShellScript -Name "PC Frontend Dependency Audit" -ScriptPath "scripts\check-dependency-audit.ps1" -Arguments @(
            "-Mode",
            "Strict",
            "-SkipRust"
        )
    }

    Invoke-NpmCommand -Name "PC Frontend Lint" -Arguments @("run", "lint")
    Invoke-NpmCommand -Name "PC Frontend Build" -Arguments @("run", "build")
    Invoke-NpmCommand -Name "PC Frontend Bundle Budget" -Arguments @("run", "check:bundle-budget")
    Invoke-NpmCommand -Name "PC Frontend Message Flow Tests" -Arguments @("run", "test:message-flow")
    Invoke-NpmCommand -Name "PC Frontend Workspace Access Tests" -Arguments @("run", "test:workspace-access")
    Invoke-NpmCommand -Name "Admin Realtime Smoke" -Arguments @("run", "test:admin-realtime")
}

Write-Host "LOCAL_QUALITY_GATES=passed scope=$Scope server=$runsServer frontend=$runsFrontend"
