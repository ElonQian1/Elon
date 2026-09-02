#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$QuantProjectPath
)

$ErrorActionPreference = "Stop"
$mainRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$quantRoot = [System.IO.Path]::GetFullPath($QuantProjectPath)

function Resolve-GitRoot {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        throw "Project path does not exist: $Path"
    }
    $resolved = (& git -C $Path rev-parse --show-toplevel).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($resolved)) {
        throw "Project path is not a Git worktree: $Path"
    }
    return [System.IO.Path]::GetFullPath($resolved)
}

function Get-RequiredHash {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$RelativePath,
        [switch]$NormalizeNewlines
    )

    $path = Join-Path $Root $RelativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required asset-view file is missing: $path"
    }
    if (-not $NormalizeNewlines) {
        return (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    $text = [IO.File]::ReadAllText($path, [Text.Encoding]::UTF8).Replace(
        ([string][char]13 + [char]10),
        [string][char]10
    )
    $bytes = [Text.Encoding]::UTF8.GetBytes($text)
    return [Convert]::ToHexString(
        [Security.Cryptography.SHA256]::HashData($bytes)
    ).ToLowerInvariant()
}

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$FailureMessage
    )

    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$FailureMessage (exit code $LASTEXITCODE)"
    }
}

$mainRoot = Resolve-GitRoot -Path $mainRoot
$quantRoot = Resolve-GitRoot -Path $quantRoot
if ($mainRoot.Equals($quantRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Main and quant paths must be different Git worktrees"
}

$sharedFiles = @(
    [pscustomobject]@{
        name = "fixture"
        path = "contracts/quant/esk-paper-cross-repo-asset-view-v1.fixture.json"
        normalize = $false
    },
    [pscustomobject]@{
        name = "projection_schema_v2"
        path = "contracts/quant/esk-paper-asset-projection-v2.schema.json"
        normalize = $true
    }
)
$hashes = [ordered]@{}
foreach ($entry in $sharedFiles) {
    $mainHash = Get-RequiredHash -Root $mainRoot -RelativePath $entry.path `
        -NormalizeNewlines:$entry.normalize
    $quantHash = Get-RequiredHash -Root $quantRoot -RelativePath $entry.path `
        -NormalizeNewlines:$entry.normalize
    if ($mainHash -ne $quantHash) {
        throw "Cross-repository bytes differ for $($entry.path)"
    }
    $hashes[$entry.name] = $mainHash
}

$fixturePath = Join-Path $mainRoot $sharedFiles[0].path
$fixture = Get-Content -Raw -LiteralPath $fixturePath | ConvertFrom-Json
$view = $fixture.expected.view
if ($fixture.schema -ne "yilong.quant.esk_paper_cross_repo_asset_view.v1" -or
    -not $fixture.test_only -or -not $fixture.paper_only -or
    $view.schema -ne "yilong.quant.esk_asset_view.v2" -or
    $view.asset.asset_id -ne "esk" -or $view.asset.symbol -ne "ESK" -or
    $view.asset.issuance_mode -ne "paper_recorded" -or
    $view.asset.chain_status -ne "not_deployed" -or
    -not $view.simulated -or $view.funds_moved -or $view.position_created -or
    $fixture.safety.chain_token_issued -or $fixture.safety.funds_moved -or
    $fixture.safety.position_created -or $fixture.safety.nav_participation -or
    $fixture.safety.trading_started -or $fixture.safety.yield_started -or
    $fixture.safety.external_network_required) {
    throw "Asset-view fixture violates the Paper-only test contract"
}

$mainCargo = Join-Path $mainRoot "scripts/cargo-dev.ps1"
$mainArguments = @(
    "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $mainCargo,
    "-BypassValidationOrchestrator", "-SkipCacheGc", "-Domain", "agent-validation", "--",
    "test", "--manifest-path", "tools/esk-paper-contract-tests/Cargo.toml", "--lib",
    "cross_repository_asset_view_fixture", "--locked", "--offline"
)
Invoke-Checked -Executable "powershell.exe" -Arguments $mainArguments `
    -FailureMessage "Main ESK asset-view serializer test failed"

$quantAcceptance = Join-Path $quantRoot "scripts/accept-esk-cross-repo-asset-view.ps1"
Invoke-Checked -Executable "pwsh.exe" `
    -Arguments @("-NoProfile", "-File", $quantAcceptance) `
    -FailureMessage "Quant ESK asset-view acceptance failed"

$mainCommit = (& git -C $mainRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw "Could not resolve main Git commit" }
$quantCommit = (& git -C $quantRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw "Could not resolve quant Git commit" }
$mainDirty = @(& git -C $mainRoot status --porcelain=v1).Count -gt 0
$quantDirty = @(& git -C $quantRoot status --porcelain=v1).Count -gt 0

$receipt = [ordered]@{
    schema = "yilong.esk.paper_cross_repo_asset_view_receipt.v1"
    status = "passed"
    tested_at_utc = [DateTimeOffset]::UtcNow.ToString("O")
    main_commit = $mainCommit
    quant_commit = $quantCommit
    main_worktree_dirty = $mainDirty
    quant_worktree_dirty = $quantDirty
    shared_sha256 = $hashes
    scope = "local_fixed_test_vector"
    trading_mode = "paper"
    displayed_balance = [ordered]@{
        total = $view.balance.total
        available = $view.balance.available
        reserved_for_sellback = $view.balance.reserved_for_sellback
        reserved_for_quant = $view.balance.reserved_for_quant
        reserved_total = $view.balance.reserved_total
        symbol = $view.asset.symbol
    }
    live_trading_enabled = $false
    chain_token_issued = $false
    funds_moved = $false
    external_network_used = $false
    production_secrets_used = $false
    checks = @(
        "shared_fixture_bytes",
        "shared_projection_schema_v2_bytes",
        "main_projection_serialization_and_signature",
        "quant_grant_and_projection_verification",
        "quant_redacted_asset_view",
        "frontend_runtime_validation",
        "react_visible_balance_render",
        "tamper_denial"
    )
}

Write-Output (
    "ESK_PAPER_CROSS_REPO_ASSET_VIEW_RECEIPT=" +
    ($receipt | ConvertTo-Json -Compress -Depth 5)
)
