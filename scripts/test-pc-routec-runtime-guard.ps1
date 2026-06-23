param(
    [switch]$SkipCargoTests
)

$ErrorActionPreference = "Stop"

$RepoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
$ServerCargo = Join-Path $RepoRoot "server\Cargo.toml"
$RuntimeChoiceModule = Join-Path $RepoRoot "server\src\pc_agent_runtime_choice.rs"
$RouteCStatusModule = Join-Path $RepoRoot "server\src\node_agent_route_c_status.rs"
$ServerRuntimeModule = Join-Path $RepoRoot "server\src\server_agent_runtime.rs"
$GuardModule = Join-Path $RepoRoot "server\src\server_agent_runtime_guard.rs"
$BudgetModule = Join-Path $RepoRoot "server\src\server_agent_runtime_budget.rs"

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

function Invoke-CargoTestFilter {
    param(
        [string]$Filter
    )

    cargo test --manifest-path $ServerCargo $Filter
    if ($LASTEXITCODE -ne 0) {
        throw "cargo test filter '$Filter' failed with exit code $LASTEXITCODE"
    }
}

Set-Location $RepoRoot

Invoke-Step "Static Route C runtime guard contract" {
    Assert-FileContains `
        -Path $RuntimeChoiceModule `
        -Needle "route_c_status_allows_selection" `
        -Message "Route C runtime choice must inspect cloud status before selecting server-runtime"
    Assert-FileContains `
        -Path $RuntimeChoiceModule `
        -Needle "auto_route_skips_route_c_when_admission_is_limited" `
        -Message "Route C runtime choice must test admission-limited fallback"
    Assert-FileContains `
        -Path $RuntimeChoiceModule `
        -Needle "auto_route_skips_route_c_when_budget_is_exhausted" `
        -Message "Route C runtime choice must test budget-exhausted fallback"
    Assert-FileContains `
        -Path $RouteCStatusModule `
        -Needle "admissionAvailability" `
        -Message "Route C node cloud status must preserve admission availability"
    Assert-FileContains `
        -Path $ServerRuntimeModule `
        -Needle "admission_availability.ready" `
        -Message "Server Route C status must include admission readiness in ready calculation"
    Assert-FileContains `
        -Path $GuardModule `
        -Needle "admission_availability_reports_capacity_reason" `
        -Message "Server Route C guard must test admission capacity reasons"
    Assert-FileContains `
        -Path $BudgetModule `
        -Needle "budget_status_reports_exhausted_per_user_daily_call_limit" `
        -Message "Server Route C budget must test per-user daily limit exhaustion"
    Assert-FileContains `
        -Path (Join-Path $RepoRoot "server\src\store\route_c_budget.rs") `
        -Needle "clean_error_summary" `
        -Message "Route C budget audit must sanitize persisted error summaries"
    Assert-FileContains `
        -Path (Join-Path $RepoRoot "server\src\store\route_c_budget.rs") `
        -Needle "secret prompt text" `
        -Message "Route C budget audit must keep a regression test for prompt-text redaction"
    Write-Host "Static Route C runtime guard contract passed."
}

if (-not $SkipCargoTests) {
    Invoke-Step "Route C runtime choice tests" {
        Invoke-CargoTestFilter "pc_agent_runtime_choice"
    }

    Invoke-Step "Route C node cloud status tests" {
        Invoke-CargoTestFilter "node_agent_route_c_status"
    }

    Invoke-Step "Server Route C admission and budget tests" {
        Invoke-CargoTestFilter "server_agent_runtime_guard"
        Invoke-CargoTestFilter "server_agent_runtime_budget"
        Invoke-CargoTestFilter "server_agent_runtime"
    }
}

Write-Host ""
Write-Host "PC Route C runtime guard gate passed." -ForegroundColor Green
