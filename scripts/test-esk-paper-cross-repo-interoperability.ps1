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
        throw "Required interoperability file is missing: $path"
    }
    if (-not $NormalizeNewlines) {
        return (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    $text = [IO.File]::ReadAllText($path, [Text.Encoding]::UTF8).Replace("`r`n", "`n")
    $bytes = [Text.Encoding]::UTF8.GetBytes($text)
    return [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
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
    [pscustomobject]@{ name = "fixture"; path = "contracts/quant/esk-paper-cross-repo-interoperability-v1.fixture.json"; normalize = $false },
    [pscustomobject]@{ name = "authorization_schema"; path = "contracts/quant/esk-paper-allocation-authorization-v1.schema.json"; normalize = $true },
    [pscustomobject]@{ name = "receipt_schema"; path = "contracts/quant/esk-paper-allocation-receipt-v1.schema.json"; normalize = $true }
)
$fixtureRelativePath = "contracts/quant/esk-paper-cross-repo-interoperability-v1.fixture.json"
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

$fixturePath = Join-Path $mainRoot $fixtureRelativePath
$fixture = Get-Content -Raw -LiteralPath $fixturePath | ConvertFrom-Json
if ($fixture.schema -ne "yilong.quant.esk_paper_cross_repo_interoperability.v1" -or
    -not $fixture.test_only -or -not $fixture.paper_only -or
    -not $fixture.expected.simulated -or $fixture.expected.funds_moved -or
    $fixture.expected.quant_units_issued -or $fixture.expected.nav_participation -or
    $fixture.expected.trading_started) {
    throw "Interoperability fixture violates the Paper-only test contract"
}

$mainCargo = Join-Path $mainRoot "scripts/cargo-dev.ps1"
$mainArguments = @(
    "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $mainCargo,
    "-BypassValidationOrchestrator", "-SkipCacheGc", "-Domain", "agent-validation", "--",
    "test", "--manifest-path", "server/Cargo.toml", "--bin", "elon-server",
    "cross_repository", "--locked", "--offline"
)
Invoke-Checked -Executable "powershell.exe" -Arguments $mainArguments `
    -FailureMessage "Main interoperability tests failed"

$quantCache = Join-Path $quantRoot "scripts/rust-cache.ps1"
$quotedQuantCache = $quantCache.Replace("'", "''")
$quantCommand = @"
& '$quotedQuantCache' run -Domain agent-validation -RemainingArgs @(
    'test', '--package', 'yilong-quant-api', '--test',
    'esk_cross_repo_interoperability', '--locked', '--offline'
)
exit `$LASTEXITCODE
"@
$encodedQuantCommand = [Convert]::ToBase64String(
    [Text.Encoding]::Unicode.GetBytes($quantCommand)
)
Invoke-Checked -Executable "pwsh.exe" `
    -Arguments @("-NoProfile", "-EncodedCommand", $encodedQuantCommand) `
    -FailureMessage "Quant interoperability tests failed"

$mainCommit = (& git -C $mainRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw "Could not resolve main Git commit" }
$quantCommit = (& git -C $quantRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw "Could not resolve quant Git commit" }
$mainDirty = @(& git -C $mainRoot status --porcelain=v1).Count -gt 0
$quantDirty = @(& git -C $quantRoot status --porcelain=v1).Count -gt 0

$receipt = [ordered]@{
    schema = "yilong.esk.paper_cross_repo_interoperability_receipt.v1"
    status = "passed"
    tested_at_utc = [DateTimeOffset]::UtcNow.ToString("O")
    main_commit = $mainCommit
    quant_commit = $quantCommit
    main_worktree_dirty = $mainDirty
    quant_worktree_dirty = $quantDirty
    shared_sha256 = $hashes
    scope = "local_fixed_test_vectors"
    trading_mode = "paper"
    live_trading_enabled = $false
    funds_moved = $false
    external_network_used = $false
    production_secrets_used = $false
    checks = @(
        "shared_fixture_bytes",
        "shared_authorization_schema_bytes",
        "shared_receipt_schema_bytes",
        "main_grant_and_authorization_serialization",
        "quant_grant_and_authorization_verification",
        "quant_accepted_and_released_receipt_serialization",
        "main_receipt_verification",
        "tamper_and_revoked_key_denial"
    )
}

Write-Output (
    "ESK_PAPER_CROSS_REPO_INTEROPERABILITY_RECEIPT=" +
    ($receipt | ConvertTo-Json -Compress -Depth 5)
)
