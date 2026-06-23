#requires -Version 7.0

param(
    [string]$RefreshPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2TokenlessRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2TokenlessPath {
    param(
        [string]$Path,
        [string]$Root
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return (Join-Path $Root $Path)
}

function Get-Fb2TokenlessProperty {
    param(
        [object]$Object,
        [string]$Name,
        [object]$Default = $null
    )

    if ($null -eq $Object) {
        return $Default
    }
    if ($Object -is [System.Collections.IDictionary] -and $Object.Contains($Name)) {
        return $Object[$Name]
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $Default
    }
    return $property.Value
}

function Read-Fb2TokenlessJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Refresh summary not found: $Path. Run scripts\fb2-ai-center-refresh-current-status.ps1 first."
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Add-Fb2TokenlessCheck {
    param(
        [System.Collections.ArrayList]$Checks,
        [string]$Name,
        [bool]$Passed,
        [string]$Details = ""
    )

    [void]$Checks.Add([ordered]@{
        name = $Name
        passed = $Passed
        details = $Details
    })
}

function Test-Fb2TokenlessSecretSafe {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) {
        return $true
    }
    if ($Text -match '(?i)FB2_AI_CENTER_TOKEN\s*=\s*["''][^<]') {
        return $false
    }
    if ($Text -match '(?i)-Fb2(AiCenter)?Token\s+(?!<FB2_AI_CENTER_TOKEN>)[^\s]+') {
        return $false
    }
    if ($Text -match '(?i)-Fb2Password\s+(?!<FB2_PASSWORD>)[^\s]+') {
        return $false
    }
    if ($Text -match '(?i)(bearer|token|password|secret)[=:]\s*(?!<)[A-Za-z0-9_\-\.]{12,}') {
        return $false
    }
    return $true
}

function Test-Fb2TokenlessContainsAll {
    param(
        [string[]]$Values,
        [string[]]$Required
    )

    foreach ($item in $Required) {
        if (-not ($Values -contains $item)) {
            return $false
        }
    }
    return $true
}

function Test-Fb2TokenlessFileExists {
    param([string]$Path)

    -not [string]::IsNullOrWhiteSpace($Path) -and (Test-Path -LiteralPath $Path)
}

function New-Fb2TokenlessContinuationValidation {
    param(
        [object]$Refresh,
        [string]$SourcePath
    )

    $checks = [System.Collections.ArrayList]::new()
    $blocking = Get-Fb2TokenlessProperty $Refresh "blocking_state"
    $commands = Get-Fb2TokenlessProperty $Refresh "next_commands"
    $completion = Get-Fb2TokenlessProperty $Refresh "completion_matrix"
    $gates = Get-Fb2TokenlessProperty $completion "gates"
    $files = Get-Fb2TokenlessProperty $Refresh "files"
    $gapBoard = Get-Fb2TokenlessProperty $Refresh "gap_action_board"
    $freshness = Get-Fb2TokenlessProperty $Refresh "evidence_freshness"
    $exportedSamples = Get-Fb2TokenlessProperty $Refresh "exported_context_pack_sample_set_validation"
    $safeWithoutSecret = @(Get-Fb2TokenlessProperty $blocking "safe_to_continue_without_secret" @()) | ForEach-Object { [string]$_ }
    $requiresSecret = @(Get-Fb2TokenlessProperty $blocking "requires_secret" @()) | ForEach-Object { [string]$_ }

    Add-Fb2TokenlessCheck $checks "refresh schema" ([string](Get-Fb2TokenlessProperty $Refresh "schema" "") -eq "fb2.main_project.status_refresh.v1")
    Add-Fb2TokenlessCheck $checks "public contract ready" ([bool](Get-Fb2TokenlessProperty $Refresh "public_contract_ready" $false))
    Add-Fb2TokenlessCheck $checks "server deploy ready" ([bool](Get-Fb2TokenlessProperty $Refresh "server_deploy_ready" $false))
    Add-Fb2TokenlessCheck $checks "data goal complete" ([bool](Get-Fb2TokenlessProperty $Refresh "data_goal_complete" $false))
    Add-Fb2TokenlessCheck $checks "full final remains incomplete" (-not [bool](Get-Fb2TokenlessProperty $Refresh "full_final_complete" $true))
    Add-Fb2TokenlessCheck $checks "token absent" (-not [bool](Get-Fb2TokenlessProperty $Refresh "token_present" $true))
    Add-Fb2TokenlessCheck $checks "next action waits for token preflight" (
        [string](Get-Fb2TokenlessProperty $Refresh "next_minimum_action" "") -eq "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
    )

    Add-Fb2TokenlessCheck $checks "completion schema" ([string](Get-Fb2TokenlessProperty $completion "schema" "") -eq "fb2.main_project.completion_matrix.v1")
    Add-Fb2TokenlessCheck $checks "completion gates match refresh" (
        [bool](Get-Fb2TokenlessProperty $gates "data_goal_complete" $false) -eq [bool](Get-Fb2TokenlessProperty $Refresh "data_goal_complete" $false) -and
        [bool](Get-Fb2TokenlessProperty $gates "full_final_complete" $true) -eq [bool](Get-Fb2TokenlessProperty $Refresh "full_final_complete" $false) -and
        [bool](Get-Fb2TokenlessProperty $gates "token_present" $true) -eq [bool](Get-Fb2TokenlessProperty $Refresh "token_present" $false)
    )
    Add-Fb2TokenlessCheck $checks "voice remains deferred" ([bool](Get-Fb2TokenlessProperty $gates "voice_deferred_by_user" $false))

    Add-Fb2TokenlessCheck $checks "blocking state uses external token" ([string](Get-Fb2TokenlessProperty $blocking "external_secret" "") -eq "FB2_AI_CENTER_TOKEN")
    Add-Fb2TokenlessCheck $checks "blocking state still blocked by token" ([bool](Get-Fb2TokenlessProperty $blocking "blocked_by_external_secret" $false))
    Add-Fb2TokenlessCheck $checks "safe without secret list complete" (Test-Fb2TokenlessContainsAll -Values $safeWithoutSecret -Required @(
            "public_contract_regression",
            "status_refresh_selftest",
            "offline_context_pack_sample_validation",
            "handoff_documentation"
        ))
    Add-Fb2TokenlessCheck $checks "requires secret list complete" (Test-Fb2TokenlessContainsAll -Values $requiresSecret -Required @(
            "live_context_pack_permission_quality_refresh",
            "current_user_order_live_verification",
            "platform_order_summary_live_verification",
            "feedback_quality_live_refresh"
        ))
    Add-Fb2TokenlessCheck $checks "safe and secret lists do not overlap" (@($safeWithoutSecret | Where-Object { $requiresSecret -contains $_ }).Count -eq 0)

    $requiredCommands = @(
        "validate_public_contract_status",
        "validate_exported_context_pack_sample_set",
        "validate_context_projection_log",
        "validate_user_scenario_audit",
        "validate_current_state",
        "validate_gap_action_board",
        "validate_handoff_prompt",
        "validate_live_preflight_request",
        "validate_tokenless_continuation",
        "no_write_direct_read",
        "data_only_preflight",
        "visible_regression_requires_authorization"
    )
    foreach ($name in $requiredCommands) {
        $command = [string](Get-Fb2TokenlessProperty $commands $name "")
        Add-Fb2TokenlessCheck $checks "next command $name exists" (-not [string]::IsNullOrWhiteSpace($command))
        Add-Fb2TokenlessCheck $checks "next command $name secret safe" (Test-Fb2TokenlessSecretSafe -Text $command)
    }

    $tokenlessCommand = [string](Get-Fb2TokenlessProperty $commands "validate_tokenless_continuation" "")
    Add-Fb2TokenlessCheck $checks "tokenless validator command targets this script" (
        $tokenlessCommand -match "validate-fb2-tokenless-continuation\.ps1" -and
        $tokenlessCommand -match "tokenless-continuation-validation-current\.json"
    )

    $projectionLogCommand = [string](Get-Fb2TokenlessProperty $commands "validate_context_projection_log" "")
    Add-Fb2TokenlessCheck $checks "context projection validator command targets log evidence" (
        $projectionLogCommand -match "validate-fb2-context-projection-log\.ps1" -and
        $projectionLogCommand -match "context-projection-log-validation-current\.json"
    )

    $userScenarioCommand = [string](Get-Fb2TokenlessProperty $commands "validate_user_scenario_audit" "")
    Add-Fb2TokenlessCheck $checks "user scenario validator command targets product audit" (
        $userScenarioCommand -match "validate-fb2-user-scenario-audit\.ps1" -and
        $userScenarioCommand -match "user-scenario-audit-validation-current\.json"
    )

    $readOnlyCommand = [string](Get-Fb2TokenlessProperty $commands "no_write_direct_read" "")
    Add-Fb2TokenlessCheck $checks "read-only command cannot write visible messages" (
        $readOnlyCommand -match "ReadOnlyDirectRead" -and
        $readOnlyCommand -notmatch "AllowVisibleMessages" -and
        $readOnlyCommand -notmatch "Fb2AiCenterToken"
    )

    $preflightCommand = [string](Get-Fb2TokenlessProperty $commands "data_only_preflight" "")
    Add-Fb2TokenlessCheck $checks "data-only preflight is no visible write" (
        $preflightCommand -match "DataOnlyAcceptance" -and
        $preflightCommand -match "PreflightOnly" -and
        $preflightCommand -match "<FB2_AI_CENTER_TOKEN>" -and
        $preflightCommand -notmatch "AllowVisibleMessages"
    )

    $visibleCommand = [string](Get-Fb2TokenlessProperty $commands "visible_regression_requires_authorization" "")
    Add-Fb2TokenlessCheck $checks "visible regression stays explicitly visible" (
        $visibleCommand -match "AllowVisibleMessages" -and
        $visibleCommand -match "<FB2_AI_CENTER_TOKEN>"
    )

    Add-Fb2TokenlessCheck $checks "gap action board schema" ([string](Get-Fb2TokenlessProperty $gapBoard "schema" "") -eq "fb2.main_project.gap_action_board.v1")
    Add-Fb2TokenlessCheck $checks "gap action board blocked by token" ([bool](Get-Fb2TokenlessProperty $gapBoard "blocked_by_external_secret" $false))
    Add-Fb2TokenlessCheck $checks "evidence freshness schema" ([string](Get-Fb2TokenlessProperty $freshness "schema" "") -eq "fb2.main_project.evidence_freshness.v1")
    Add-Fb2TokenlessCheck $checks "exported samples have status" ($null -ne $exportedSamples)
    if ($null -ne $exportedSamples) {
        $attempted = [bool](Get-Fb2TokenlessProperty $exportedSamples "attempted" $false)
        $complete = [bool](Get-Fb2TokenlessProperty $exportedSamples "complete" $false)
        $skippedReason = [string](Get-Fb2TokenlessProperty $exportedSamples "skipped_reason" "")
        Add-Fb2TokenlessCheck $checks "exported samples complete or explicitly skipped" (
            ($attempted -and $complete) -or
            (-not $attempted -and -not [string]::IsNullOrWhiteSpace($skippedReason))
        )
    }

    foreach ($name in @("status_refresh", "status", "goal_audit", "handoff_prompt")) {
        $path = [string](Get-Fb2TokenlessProperty $files $name "")
        Add-Fb2TokenlessCheck $checks "file $name exists" (Test-Fb2TokenlessFileExists -Path $path) $path
    }

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.tokenless_continuation_validation.v1"
        source_refresh = $SourcePath
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
        token_present = [bool](Get-Fb2TokenlessProperty $Refresh "token_present" $false)
        data_goal_complete = [bool](Get-Fb2TokenlessProperty $Refresh "data_goal_complete" $false)
        full_final_complete = [bool](Get-Fb2TokenlessProperty $Refresh "full_final_complete" $false)
        next_minimum_action = [string](Get-Fb2TokenlessProperty $Refresh "next_minimum_action" "")
    }
}

function New-Fb2TokenlessFixture {
    param(
        [string]$TempRoot,
        [switch]$BadTokenPresent,
        [switch]$BadVisiblePreflight,
        [switch]$BadSecretLeak,
        [switch]$BadMissingSafeItem,
        [switch]$BadPublicContract
    )

    $commandToken = if ($BadSecretLeak) { "real-secret-token-1234567890" } else { "<FB2_AI_CENTER_TOKEN>" }
    $safeList = if ($BadMissingSafeItem) {
        @("public_contract_regression", "status_refresh_selftest")
    } else {
        @("public_contract_regression", "status_refresh_selftest", "offline_context_pack_sample_validation", "handoff_documentation")
    }
    $dataOnlyCommand = if ($BadVisiblePreflight) {
        "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -AllowVisibleMessages -Fb2AiCenterToken $commandToken"
    } else {
        "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2AiCenterToken $commandToken"
    }

    $statusPath = Join-Path $TempRoot "status-current.json"
    $goalPath = Join-Path $TempRoot "goal-audit-current.json"
    $handoffPromptPath = Join-Path $TempRoot "handoff-prompt-current.md"
    $refreshPath = Join-Path $TempRoot "status-refresh-current.json"
    Set-Content -LiteralPath $statusPath -Value "{}" -Encoding UTF8
    Set-Content -LiteralPath $goalPath -Value "{}" -Encoding UTF8
    Set-Content -LiteralPath $handoffPromptPath -Value "# prompt" -Encoding UTF8

    [pscustomobject]@{
        schema = "fb2.main_project.status_refresh.v1"
        public_contract_ready = -not [bool]$BadPublicContract
        server_deploy_ready = $true
        data_goal_complete = $true
        full_final_complete = $false
        token_present = [bool]$BadTokenPresent
        next_minimum_action = "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
        files = [ordered]@{
            status_refresh = $refreshPath
            status = $statusPath
            goal_audit = $goalPath
            handoff_prompt = $handoffPromptPath
        }
        blocking_state = [ordered]@{
            blocked_by_external_secret = $true
            external_secret = "FB2_AI_CENTER_TOKEN"
            safe_to_continue_without_secret = $safeList
            requires_secret = @(
                "live_context_pack_permission_quality_refresh",
                "current_user_order_live_verification",
                "platform_order_summary_live_verification",
                "feedback_quality_live_refresh"
            )
        }
        next_commands = [ordered]@{
            validate_public_contract_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-public-contract-status.ps1 -OutputPath target\fb2-ai-center\public-contract-status-current.json"
            validate_exported_context_pack_sample_set = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir <fb2_repo>\target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\fb2-repo-context-pack-samples-validation-current.json"
            validate_context_projection_log = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-projection-log.ps1 -StatusPath target\fb2-ai-center\status-current.json -OutputPath target\fb2-ai-center\context-projection-log-validation-current.json"
            validate_user_scenario_audit = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-user-scenario-audit.ps1 -StatusPath target\fb2-ai-center\status-current.json -OutputPath target\fb2-ai-center\user-scenario-audit-validation-current.json"
            validate_current_state = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-current-state.ps1"
            validate_gap_action_board = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-gap-action-board.ps1"
            validate_handoff_prompt = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-handoff-prompt.ps1"
            validate_live_preflight_request = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-live-preflight-request.ps1 -StatusPath target\fb2-ai-center\status-current.json"
            validate_tokenless_continuation = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-tokenless-continuation.ps1 -OutputPath target\fb2-ai-center\tokenless-continuation-validation-current.json"
            no_write_direct_read = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead -Fb2Password <FB2_PASSWORD>"
            data_only_preflight = $dataOnlyCommand
            visible_regression_requires_authorization = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -AllowVisibleMessages -Fb2AiCenterToken $commandToken"
        }
        completion_matrix = [ordered]@{
            schema = "fb2.main_project.completion_matrix.v1"
            gates = [ordered]@{
                data_goal_complete = $true
                full_final_complete = $false
                token_present = [bool]$BadTokenPresent
                voice_deferred_by_user = $true
            }
        }
        gap_action_board = [ordered]@{
            schema = "fb2.main_project.gap_action_board.v1"
            blocked_by_external_secret = $true
        }
        evidence_freshness = [ordered]@{
            schema = "fb2.main_project.evidence_freshness.v1"
        }
        exported_context_pack_sample_set_validation = [ordered]@{
            attempted = $true
            complete = $true
            skipped_reason = ""
        }
    }
}

function Invoke-Fb2TokenlessSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-tokenless-continuation-selftest-" + [guid]::NewGuid().ToString("N"))
    try {
        New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
        $failed = 0
        $good = New-Fb2TokenlessFixture -TempRoot $tempRoot
        $good.files.status_refresh = Join-Path $tempRoot "status-refresh-current.json"
        Set-Content -LiteralPath ([string]$good.files.status_refresh) -Value ($good | ConvertTo-Json -Depth 8) -Encoding UTF8
        $goodResult = New-Fb2TokenlessContinuationValidation -Refresh $good -SourcePath "selftest-good.json"
        if (-not [bool]$goodResult.success) {
            $goodResult | ConvertTo-Json -Depth 8
            $failed++
        }

        foreach ($case in @(
                @{ name = "token-present"; args = @{ BadTokenPresent = $true } },
                @{ name = "visible-preflight"; args = @{ BadVisiblePreflight = $true } },
                @{ name = "secret-leak"; args = @{ BadSecretLeak = $true } },
                @{ name = "missing-safe"; args = @{ BadMissingSafeItem = $true } },
                @{ name = "public-contract"; args = @{ BadPublicContract = $true } }
            )) {
            $caseArgs = [hashtable]$case.args
            $fixture = New-Fb2TokenlessFixture -TempRoot $tempRoot @caseArgs
            $fixture.files.status_refresh = Join-Path $tempRoot ("status-refresh-" + [string]$case.name + ".json")
            Set-Content -LiteralPath ([string]$fixture.files.status_refresh) -Value ($fixture | ConvertTo-Json -Depth 8) -Encoding UTF8
            $result = New-Fb2TokenlessContinuationValidation -Refresh $fixture -SourcePath ("selftest-" + [string]$case.name + ".json")
            if ([bool]$result.success) {
                $failed++
            }
        }

        Write-Output "== SelfTest Summary =="
        Write-Output "failed=$failed"
        if ($failed -gt 0) {
            exit 1
        }
    } finally {
        if (Test-Path -LiteralPath $tempRoot) {
            Remove-Item -LiteralPath $tempRoot -Recurse -Force
        }
    }
}

if ($SelfTest) {
    Invoke-Fb2TokenlessSelfTest
    exit 0
}

$root = Get-Fb2TokenlessRepoRoot
if ([string]::IsNullOrWhiteSpace($RefreshPath)) {
    $RefreshPath = Join-Path $root "target\fb2-ai-center\status-refresh-current.json"
} else {
    $RefreshPath = Resolve-Fb2TokenlessPath -Path $RefreshPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\tokenless-continuation-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2TokenlessPath -Path $OutputPath -Root $root
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$refresh = Read-Fb2TokenlessJson -Path $RefreshPath
$result = New-Fb2TokenlessContinuationValidation -Refresh $refresh -SourcePath $RefreshPath
$json = $result | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $OutputPath -Value $json -Encoding UTF8
$json

if (-not [bool]$result.success) {
    exit 1
}
