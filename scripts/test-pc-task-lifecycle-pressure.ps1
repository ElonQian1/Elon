param(
    [switch]$SkipJournalUnitTests,
    [switch]$SkipPressureTests
)

$ErrorActionPreference = "Stop"

$RepoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
$ServerCargo = Join-Path $RepoRoot "server\Cargo.toml"
$NodeAgentMain = Join-Path $RepoRoot "server\src\node_agent_main.rs"
$PressureModule = Join-Path $RepoRoot "server\src\node_agent_task_lifecycle_pressure_tests.rs"
$ProjectAgentRunsModule = Join-Path $RepoRoot "server\src\node_agent_project_agent_runs.rs"

function Invoke-Step {
    param(
        [string]$Label,
        [scriptblock]$Body
    )

    Write-Host ""
    Write-Host "== $Label ==" -ForegroundColor Cyan
    & $Body
}

function Assert-FileContains {
    param(
        [string]$Path,
        [string]$Needle,
        [string]$Message
    )

    $text = Get-Content -LiteralPath $Path -Raw
    if (-not $text.Contains($Needle)) {
        throw $Message
    }
}

Set-Location $RepoRoot

Invoke-Step "Static pressure-test contract" {
    if (-not (Test-Path -LiteralPath $PressureModule)) {
        throw "Missing node_agent_task_lifecycle_pressure_tests.rs"
    }
    Assert-FileContains `
        -Path $NodeAgentMain `
        -Needle "mod node_agent_task_lifecycle_pressure_tests;" `
        -Message "node_agent_task_lifecycle_pressure_tests module is not wired into node-agent tests"
    Assert-FileContains `
        -Path $PressureModule `
        -Needle "stress_concurrent_task_journal_writes_keep_registry_and_events_consistent" `
        -Message "Concurrent task-journal pressure test is missing"
    Assert-FileContains `
        -Path $PressureModule `
        -Needle "stress_restart_resume_contract_never_claims_lost_control_handles" `
        -Message "Restart resume pressure test is missing"
    Assert-FileContains `
        -Path $PressureModule `
        -Needle "stress_active_registry_rejects_duplicate_handles_and_cleans_up" `
        -Message "Active registry duplicate-handle pressure test is missing"
    Assert-FileContains `
        -Path $ProjectAgentRunsModule `
        -Needle "stress_agent_run_summary_reads_long_run_to_terminal_status" `
        -Message "Long agent-run lifecycle summary pressure test is missing"
    Write-Host "Static pressure-test contract passed."
}

if (-not $SkipJournalUnitTests) {
    Invoke-Step "Task journal unit tests" {
        cargo test --manifest-path $ServerCargo node_agent_task_journal -- --nocapture
        if ($LASTEXITCODE -ne 0) {
            throw "Task journal unit tests failed with exit code $LASTEXITCODE"
        }
    }
}

if (-not $SkipPressureTests) {
    Invoke-Step "Agent run lifecycle summary pressure tests" {
        cargo test --manifest-path $ServerCargo node_agent_project_agent_runs -- --nocapture
        if ($LASTEXITCODE -ne 0) {
            throw "Agent run lifecycle summary pressure tests failed with exit code $LASTEXITCODE"
        }
    }
}

if (-not $SkipPressureTests) {
    Invoke-Step "Task lifecycle pressure tests" {
        cargo test --manifest-path $ServerCargo node_agent_task_lifecycle_pressure_tests -- --nocapture
        if ($LASTEXITCODE -ne 0) {
            throw "Task lifecycle pressure tests failed with exit code $LASTEXITCODE"
        }
    }
}

Write-Host ""
Write-Host "PC task lifecycle pressure gate passed." -ForegroundColor Green
