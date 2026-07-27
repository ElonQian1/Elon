param(
    [string]$WorkflowPath = ".github\workflows\ci.yml"
)

$ErrorActionPreference = "Stop"

function Stop-CiQualityGateGuard {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Get-RepoRoot {
    $root = (& git rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($root)) {
        Stop-CiQualityGateGuard "Current directory is not inside a git repository."
    }
    return $root
}

function Read-TextFile {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-CiQualityGateGuard "Required file is missing: $Path"
    }
    return [System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::UTF8)
}

function Assert-Contains {
    param(
        [string]$Label,
        [string]$Text,
        [string]$Needle
    )
    if (-not $Text.Contains($Needle)) {
        Stop-CiQualityGateGuard "$Label is missing required CI entry: $Needle"
    }
}

$repoRoot = Get-RepoRoot
Set-Location $repoRoot

$workflowFullPath = Join-Path $repoRoot $WorkflowPath
$workflow = Read-TextFile $workflowFullPath

$requiredEntries = @(
    "on:",
    "push:",
    "pull_request:",
    "permissions:",
    "contents: read",
    "concurrency:",
    "cancel-in-progress: true",
    "rust-server:",
    "name: Rust Server",
    "runs-on: windows-latest",
    "RUST_TEST_THREADS: 1",
    "Source Size Guard",
    "scripts\check-source-size.ps1",
    "Release Runbook Guard",
    "scripts\check-release-runbook.ps1",
    "CI Quality Gates Guard",
    "scripts\check-ci-quality-gates.ps1",
    "Realtime Runbook Guard",
    "scripts\check-realtime-runbook.ps1",
    "Realtime Ownership Guard",
    "scripts\check-realtime-ownership.ps1",
    "Realtime Diagnostics Snapshot Guard",
    "scripts\check-realtime-diagnostics-snapshot.ps1",
    "Cache Cargo Audit",
    "actions/cache@v4",
    "~\.cargo\bin\cargo-audit.exe",
    "~\.cargo\advisory-db",
    'cargo-audit-windows-0.22.2-${{ runner.arch }}',
    "Install Cargo Audit",
    "scripts\install-cargo-audit.ps1 -Version 0.22.2",
    "Dependency Audit Report",
    "scripts\check-dependency-audit.ps1 -Mode Strict -SkipNpm -RequireRustAudit -AllowStaleRustAdvisoryDb",
    "Rust Warning Budget",
    "scripts\check-rust-warning-budget.ps1 -MaxWarnings 0",
    "Cargo Test",
    "id: cargo-test",
    "scripts\cargo-dev.ps1 test --manifest-path server\Cargo.toml",
    "Upload Rust Validation Evidence",
    "if: failure() && steps.cargo-test.outcome == 'failure'",
    "actions/upload-artifact@v4",
    'rust-validation-evidence-${{ github.run_id }}',
    "~\AppData\Local\Elon\rust-cache-v2\validation-v1\evidence",
    "retention-days: 7",
    "pc-frontend:",
    "name: PC Frontend",
    "runs-on: ubuntu-latest",
    "node-version: 22.12.0",
    "cache-dependency-path: pc-frontend/package-lock.json",
    "Install Dependencies",
    "npm ci",
    "../scripts/check-dependency-audit.ps1 -Mode Strict -SkipRust",
    "Lint",
    "npm run lint",
    "Build",
    "npm run build",
    "Bundle Budget",
    "npm run check:bundle-budget",
    "Message Flow Tests",
    "npm run test:message-flow",
    "Workspace Access Tests",
    "npm run test:workspace-access",
    "Admin Realtime Smoke",
    "npm run test:admin-realtime"
)

foreach ($entry in $requiredEntries) {
    Assert-Contains -Label $WorkflowPath -Text $workflow -Needle $entry
}

Write-Host "CI_QUALITY_GATES_GUARD=passed entries=$($requiredEntries.Count)"
