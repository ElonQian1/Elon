param(
    [string]$ProviderUserId,
    [string]$ProviderAccount,
    [string]$NodeUrl = "http://127.0.0.1:7799",
    [switch]$RunRealCli,
    [switch]$SkipCargo,
    [switch]$SkipFakeCli,
    [switch]$DetailedCargo,
    [int]$CargoLockTimeoutSeconds = 120
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$Results = New-Object System.Collections.Generic.List[object]
$Failed = $false

function Add-StepResult {
    param(
        [string]$Name,
        [string]$Kind,
        [bool]$Ok,
        [string]$Evidence,
        [string]$ErrorMessage = ""
    )
    $script:Results.Add([pscustomobject]@{
        name = $Name
        kind = $Kind
        ok = $Ok
        evidence = $Evidence
        error = $ErrorMessage
    }) | Out-Null
    if (-not $Ok) {
        $script:Failed = $true
    }
}

function Invoke-CheckedScript {
    param(
        [string]$Name,
        [string]$Kind,
        [scriptblock]$Body,
        [string]$Evidence
    )
    try {
        Push-Location $RepoRoot
        try {
            & $Body
            Add-StepResult -Name $Name -Kind $Kind -Ok $true -Evidence $Evidence
        } finally {
            Pop-Location
        }
    } catch {
        Add-StepResult -Name $Name -Kind $Kind -Ok $false -Evidence $Evidence -ErrorMessage ($_.Exception.Message)
    }
}

function Invoke-CargoFilter {
    param(
        [string]$Name,
        [string]$Filter,
        [string]$Evidence
    )
    Invoke-CheckedScript -Name $Name -Kind "cargo_test" -Evidence $Evidence -Body {
        & (Join-Path $PSScriptRoot "cargo-dev.ps1") -LockTimeoutSeconds $CargoLockTimeoutSeconds test --manifest-path (Join-Path $RepoRoot "server\Cargo.toml") $Filter
        if ($LASTEXITCODE -ne 0) {
            throw "cargo test filter '$Filter' failed with exit code $LASTEXITCODE"
        }
    }
}

if (-not $SkipFakeCli) {
    Invoke-CheckedScript `
        -Name "fake_cli_child_codex_home" `
        -Kind "fake_cli" `
        -Evidence "Fake CLI verifies managed CODEX_HOME propagation and expired lease fallback without spending real quota." `
        -Body {
            & (Join-Path $PSScriptRoot "test-codex-vault-fake-cli-env.ps1") | Out-Host
            if ($null -ne $LASTEXITCODE -and $LASTEXITCODE -ne 0) {
                throw "fake CLI env probe failed with exit code $LASTEXITCODE"
            }
        }
}

if (-not $SkipCargo) {
    if ($DetailedCargo) {
        Invoke-CargoFilter `
            -Name "restart_after_expired_lease" `
            -Filter "codex_child_home" `
            -Evidence "New child processes do not keep using expired managed shared CODEX_HOME."
        Invoke-CargoFilter `
            -Name "grant_revoke_and_expiry" `
            -Filter "revoked_and_expired_grants_are_not_shareable" `
            -Evidence "Revoked or expired sharing grants cannot create usable leases."
        Invoke-CargoFilter `
            -Name "reciprocal_concurrent_billing_isolated" `
            -Filter "reciprocal_shared_codex_usage_keeps_audit_chain_separate" `
            -Evidence "Reciprocal sharing keeps provider, consumer, lease, and billing chains isolated."
        Invoke-CargoFilter `
            -Name "platform_shared_codex_billing_e2e" `
            -Filter "platform_shared_codex_task_billing_links_full_lease_audit_chain" `
            -Evidence "Platform shared_codex usage links token usage, billing event, node transaction, and idempotent replay."
        Invoke-CargoFilter `
            -Name "sharing_health_alerts" `
            -Filter "sharing_health_flags_expired_uncleared_and_accounting_anomalies" `
            -Evidence "Expired uncleared leases, accounting gaps, and failure events surface in sharing health."
    } else {
        Invoke-CargoFilter `
            -Name "rust_codex_sharing_matrix" `
            -Filter "codex" `
            -Evidence "Aggregated Codex tests cover child CODEX_HOME selection, grant revoke/expiry, reciprocal sharing, shared_codex billing, and admin/health alerts. Pass -DetailedCargo for split steps."
    }
}

if ($RunRealCli) {
    if ([string]::IsNullOrWhiteSpace($ProviderUserId) -and [string]::IsNullOrWhiteSpace($ProviderAccount)) {
        Add-StepResult `
            -Name "real_cli_bad_auth_and_shared_auth" `
            -Kind "real_cli" `
            -Ok $false `
            -Evidence "Real Codex CLI switch requires -ProviderUserId or -ProviderAccount." `
            -ErrorMessage "missing provider"
    } else {
        Invoke-CheckedScript `
            -Name "real_cli_bad_auth_and_shared_auth" `
            -Kind "real_cli" `
            -Evidence "Real bad auth must fail, shared auth must succeed, and the managed lease must be cleared." `
            -Body {
                $args = @(
                    "-NodeUrl", $NodeUrl
                )
                if (-not [string]::IsNullOrWhiteSpace($ProviderUserId)) {
                    $args += @("-ProviderUserId", $ProviderUserId)
                }
                if (-not [string]::IsNullOrWhiteSpace($ProviderAccount)) {
                    $args += @("-ProviderAccount", $ProviderAccount)
                }
                & (Join-Path $PSScriptRoot "test-codex-vault-sharing-switch.ps1") @args | Out-Host
                if ($LASTEXITCODE -ne 0) {
                    throw "real CLI sharing switch failed with exit code $LASTEXITCODE"
                }
            }
    }
} else {
    Add-StepResult `
        -Name "real_cli_bad_auth_and_shared_auth" `
        -Kind "real_cli" `
        -Ok $true `
        -Evidence "Real Codex CLI switch skipped by default; pass -RunRealCli when quota-spending proof is required."
}

$Summary = [pscustomobject]@{
    ok = (-not $Failed)
    run_real_cli = [bool]$RunRealCli
    steps = $Results
}
$Summary | ConvertTo-Json -Depth 8
if ($Failed) {
    exit 1
}
