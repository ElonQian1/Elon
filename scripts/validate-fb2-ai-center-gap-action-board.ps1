#requires -Version 7.0

param(
    [string]$RefreshPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2GapRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2GapPath {
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

function Get-Fb2GapProperty {
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

function Read-Fb2GapJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Refresh summary not found: $Path. Run scripts\fb2-ai-center-refresh-current-status.ps1 first."
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Add-Fb2GapCheck {
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

function Test-Fb2GapSecretSafe {
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
    return $true
}

function Find-Fb2GapAction {
    param(
        [object[]]$Actions,
        [string]$Id
    )

    @($Actions | Where-Object { [string]$_.id -eq $Id } | Select-Object -First 1)
}

function New-Fb2GapValidation {
    param(
        [object]$Refresh,
        [string]$SourcePath
    )

    $checks = [System.Collections.ArrayList]::new()
    $board = Get-Fb2GapProperty $Refresh "gap_action_board"
    $blocking = Get-Fb2GapProperty $Refresh "blocking_state"
    $ownerNextActions = Get-Fb2GapProperty $Refresh "owner_next_actions"
    $nextCommands = Get-Fb2GapProperty $Refresh "next_commands"
    $actions = @(Get-Fb2GapProperty $board "actions" @())

    Add-Fb2GapCheck $checks "gap board schema" ([string](Get-Fb2GapProperty $board "schema" "") -eq "fb2.main_project.gap_action_board.v1")
    Add-Fb2GapCheck $checks "action count matches" ([int](Get-Fb2GapProperty $board "action_count" 0) -eq @($actions).Count) ("declared=$([int](Get-Fb2GapProperty $board 'action_count' 0)) actual=$(@($actions).Count)")
    Add-Fb2GapCheck $checks "has actions" (@($actions).Count -gt 0)

    Add-Fb2GapCheck $checks "blocking state present" ($null -ne $blocking)
    if ($null -ne $blocking) {
        $safeWithoutSecret = @(Get-Fb2GapProperty $blocking "safe_to_continue_without_secret" @()) | ForEach-Object { [string]$_ }
        $requiresSecret = @(Get-Fb2GapProperty $blocking "requires_secret" @()) | ForEach-Object { [string]$_ }
        $deferredByUser = @(Get-Fb2GapProperty $blocking "deferred_by_user" @()) | ForEach-Object { [string]$_ }
        Add-Fb2GapCheck $checks "blocking state uses external token" ([string](Get-Fb2GapProperty $blocking "external_secret" "") -eq "FB2_AI_CENTER_TOKEN")
        Add-Fb2GapCheck $checks "blocking state marks token blocker" ([bool](Get-Fb2GapProperty $blocking "blocked_by_external_secret" $false))
        Add-Fb2GapCheck $checks "blocking next action matches gap board" (
            [string](Get-Fb2GapProperty $blocking "next_minimum_action" "") -eq [string](Get-Fb2GapProperty $board "next_minimum_action" "")
        )

        foreach ($item in @(
                "public_contract_regression",
                "status_refresh_selftest",
                "offline_context_pack_sample_validation",
                "handoff_documentation"
            )) {
            Add-Fb2GapCheck $checks "blocking safe without secret $item" ($safeWithoutSecret -contains $item)
        }

        foreach ($item in @(
                "live_context_pack_permission_quality_refresh",
                "current_user_order_live_verification",
                "platform_order_summary_live_verification",
                "feedback_quality_live_refresh"
            )) {
            Add-Fb2GapCheck $checks "blocking requires secret $item" ($requiresSecret -contains $item)
        }

        Add-Fb2GapCheck $checks "blocking records voice defer" ($deferredByUser -contains "ASR_TTS_final_evidence")
    }

    Add-Fb2GapCheck $checks "owner next actions present" ($null -ne $ownerNextActions)
    if ($null -ne $ownerNextActions) {
        $mainProjectAction = [string](Get-Fb2GapProperty $ownerNextActions "main_project" "")
        $fb2ProjectAction = [string](Get-Fb2GapProperty $ownerNextActions "fb2_project" "")
        $sharedAction = [string](Get-Fb2GapProperty $ownerNextActions "shared" "")
        Add-Fb2GapCheck $checks "owner main project keeps regressions green" (
            $mainProjectAction -match "contract" -and
            $mainProjectAction -match "status" -and
            $mainProjectAction -match "FB2_AI_CENTER_TOKEN"
        )
        Add-Fb2GapCheck $checks "owner fb2 project provides token or live evidence" (
            $fb2ProjectAction -match "FB2_AI_CENTER_TOKEN" -and
            $fb2ProjectAction -match "live_Context_Pack|Context_Pack|Context Pack|permission|quality"
        )
        Add-Fb2GapCheck $checks "owner shared runs token preflight" (
            $sharedAction -match "DataOnlyAcceptance_PreflightOnly" -and
            $sharedAction -match "token|FB2_AI_CENTER_TOKEN"
        )
        Add-Fb2GapCheck $checks "owner actions secret safe" (
            (Test-Fb2GapSecretSafe -Text $mainProjectAction) -and
            (Test-Fb2GapSecretSafe -Text $fb2ProjectAction) -and
            (Test-Fb2GapSecretSafe -Text $sharedAction)
        )
    }

    Add-Fb2GapCheck $checks "next commands present" ($null -ne $nextCommands)
    if ($null -ne $nextCommands) {
        $requiredCommands = @(
            "refresh_status",
            "read_status_refresh",
            "generate_context_pack_sample_request",
            "validate_context_pack_sample_set",
            "validate_exported_context_pack_sample_set",
            "validate_context_projection_log",
            "validate_current_state",
            "validate_public_contract_status",
            "validate_server_deploy_status",
            "validate_read_only_direct_read",
            "validate_gap_action_board",
            "validate_evidence_freshness",
            "validate_completion_matrix",
            "validate_handoff_prompt",
            "validate_visible_answer_policy",
            "validate_live_preflight_request",
            "validate_tokenless_continuation",
            "no_write_direct_read",
            "data_only_preflight",
            "visible_regression_requires_authorization"
        )
        foreach ($name in $requiredCommands) {
            $command = [string](Get-Fb2GapProperty $nextCommands $name "")
            Add-Fb2GapCheck $checks "next command $name exists" (-not [string]::IsNullOrWhiteSpace($command))
            Add-Fb2GapCheck $checks "next command $name secret safe" (Test-Fb2GapSecretSafe -Text $command)
        }

        $sampleRequestCommand = [string](Get-Fb2GapProperty $nextCommands "generate_context_pack_sample_request" "")
        Add-Fb2GapCheck $checks "next command sample request prints export request" ($sampleRequestCommand -match "PrintExportRequest")

        $sampleSetCommand = [string](Get-Fb2GapProperty $nextCommands "validate_context_pack_sample_set" "")
        Add-Fb2GapCheck $checks "next command sample set validates samples" ($sampleSetCommand -match "ValidateSampleSet")

        $exportedSampleCommand = [string](Get-Fb2GapProperty $nextCommands "validate_exported_context_pack_sample_set" "")
        Add-Fb2GapCheck $checks "next command exported sample set keeps fb2 repo placeholder" (
            $exportedSampleCommand -match "ValidateSampleSet" -and
            $exportedSampleCommand -match "<fb2_repo>"
        )

        $projectionLogCommand = [string](Get-Fb2GapProperty $nextCommands "validate_context_projection_log" "")
        Add-Fb2GapCheck $checks "next command context projection validates log evidence" (
            $projectionLogCommand -match "validate-fb2-context-projection-log\.ps1" -and
            $projectionLogCommand -match "context-projection-log-validation-current\.json"
        )

        $publicContractCommand = [string](Get-Fb2GapProperty $nextCommands "validate_public_contract_status" "")
        Add-Fb2GapCheck $checks "next command public contract validates public status" (
            $publicContractCommand -match "fb2-public-contract-status\.ps1" -and
            $publicContractCommand -match "public-contract-status-current\.json"
        )

        $visiblePolicyCommand = [string](Get-Fb2GapProperty $nextCommands "validate_visible_answer_policy" "")
        Add-Fb2GapCheck $checks "next command visible policy uses summary placeholder" ($visiblePolicyCommand -match "<DATA_ONLY_ACCEPTANCE_JSON>")

        $livePreflightCommand = [string](Get-Fb2GapProperty $nextCommands "validate_live_preflight_request" "")
        Add-Fb2GapCheck $checks "next command live preflight validates request" (
            $livePreflightCommand -match "validate-fb2-live-preflight-request\.ps1" -and
            $livePreflightCommand -match "status-current\.json"
        )

        $tokenlessCommand = [string](Get-Fb2GapProperty $nextCommands "validate_tokenless_continuation" "")
        Add-Fb2GapCheck $checks "next command tokenless continuation validates boundary" (
            $tokenlessCommand -match "validate-fb2-tokenless-continuation\.ps1" -and
            $tokenlessCommand -match "tokenless-continuation-validation-current\.json"
        )

        $readOnlyCommand = [string](Get-Fb2GapProperty $nextCommands "no_write_direct_read" "")
        Add-Fb2GapCheck $checks "next command read only direct read has no write flag" (
            $readOnlyCommand -match "ReadOnlyDirectRead" -and
            $readOnlyCommand -notmatch "AllowVisibleMessages" -and
            $readOnlyCommand -notmatch "Fb2AiCenterToken"
        )

        $preflightCommand = [string](Get-Fb2GapProperty $nextCommands "data_only_preflight" "")
        Add-Fb2GapCheck $checks "next command data preflight is no visible write" (
            $preflightCommand -match "DataOnlyAcceptance" -and
            $preflightCommand -match "PreflightOnly" -and
            $preflightCommand -match "<FB2_AI_CENTER_TOKEN>" -and
            $preflightCommand -notmatch "AllowVisibleMessages"
        )

        $visibleCommand = [string](Get-Fb2GapProperty $nextCommands "visible_regression_requires_authorization" "")
        Add-Fb2GapCheck $checks "next command visible regression requires authorization" (
            $visibleCommand -match "DataOnlyAcceptance" -and
            $visibleCommand -match "AllowVisibleMessages" -and
            $visibleCommand -match "<FB2_AI_CENTER_TOKEN>"
        )
    }

    foreach ($action in $actions) {
        $id = [string](Get-Fb2GapProperty $action "id" "")
        Add-Fb2GapCheck $checks "action $id has owner" (-not [string]::IsNullOrWhiteSpace([string](Get-Fb2GapProperty $action "owner" "")))
        Add-Fb2GapCheck $checks "action $id has evidence_needed" (-not [string]::IsNullOrWhiteSpace([string](Get-Fb2GapProperty $action "evidence_needed" "")))
        Add-Fb2GapCheck $checks "action $id command secret safe" (Test-Fb2GapSecretSafe -Text ([string](Get-Fb2GapProperty $action "command" "")))
        Add-Fb2GapCheck $checks "action $id notes secret safe" (Test-Fb2GapSecretSafe -Text ([string](Get-Fb2GapProperty $action "notes" "")))
    }

    $tokenAction = Find-Fb2GapAction -Actions $actions -Id "FB2_AI_CENTER_TOKEN_live_permission_quality_refresh"
    Add-Fb2GapCheck $checks "token refresh action exists" (@($tokenAction).Count -gt 0)
    if (@($tokenAction).Count -gt 0) {
        $command = [string](Get-Fb2GapProperty $tokenAction[0] "command" "")
        Add-Fb2GapCheck $checks "token refresh action blocked by secret" ([string](Get-Fb2GapProperty $tokenAction[0] "status" "") -eq "blocked_by_external_secret")
        Add-Fb2GapCheck $checks "token refresh action no write group" (-not [bool](Get-Fb2GapProperty $tokenAction[0] "requires_visible_group_write" $true))
        Add-Fb2GapCheck $checks "token refresh action requires secret" (-not [bool](Get-Fb2GapProperty $tokenAction[0] "can_run_without_secret" $true))
        Add-Fb2GapCheck $checks "token refresh command is preflight" ($command -match "DataOnlyAcceptance" -and $command -match "PreflightOnly")
        Add-Fb2GapCheck $checks "token refresh command has placeholder" ($command -match "<FB2_AI_CENTER_TOKEN>")
    }

    foreach ($id in @("voice_final_evidence", "ASR_TTS_final_evidence")) {
        $action = Find-Fb2GapAction -Actions $actions -Id $id
        if (@($action).Count -gt 0) {
            $status = [string](Get-Fb2GapProperty $action[0] "status" "")
            Add-Fb2GapCheck $checks "$id is deferred" ($status -match "^deferred")
            Add-Fb2GapCheck $checks "$id owned by pause" ([string](Get-Fb2GapProperty $action[0] "owner" "") -eq "paused_by_user")
            Add-Fb2GapCheck $checks "$id has no command" ([string]::IsNullOrWhiteSpace([string](Get-Fb2GapProperty $action[0] "command" "")))
        }
    }

    $fullFinal = Find-Fb2GapAction -Actions $actions -Id "full_final_acceptance_same_batch_voice_and_visible_chat"
    if (@($fullFinal).Count -gt 0) {
        Add-Fb2GapCheck $checks "full final requires visible group write" ([bool](Get-Fb2GapProperty $fullFinal[0] "requires_visible_group_write" $false))
        Add-Fb2GapCheck $checks "full final waits on voice/authorization" ([string](Get-Fb2GapProperty $fullFinal[0] "status" "") -match "voice|visible")
    }

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.gap_action_board_validation.v1"
        source_refresh = $SourcePath
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
    }
}

function Invoke-Fb2GapSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-gap-action-selftest-" + [guid]::NewGuid().ToString("N"))
    $refreshPath = Join-Path $tempRoot "status-refresh-current.json"
    try {
        New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
        $fixture = [pscustomobject]@{
            owner_next_actions = [ordered]@{
                main_project = "keep_contract_and_status_regressions_green_until_FB2_AI_CENTER_TOKEN_is_available"
                fb2_project = "provide_FB2_AI_CENTER_TOKEN_or_export_equivalent_live_Context_Pack_permission_quality_evidence"
                shared = "run_DataOnlyAcceptance_PreflightOnly_with_token_then_refresh_status_refresh_current_json"
            }
            blocking_state = [ordered]@{
                blocked_by_external_secret = $true
                external_secret = "FB2_AI_CENTER_TOKEN"
                deferred_by_user = @("ASR_TTS_final_evidence")
                safe_to_continue_without_secret = @(
                    "public_contract_regression",
                    "status_refresh_selftest",
                    "offline_context_pack_sample_validation",
                    "handoff_documentation"
                )
                requires_secret = @(
                    "live_context_pack_permission_quality_refresh",
                    "current_user_order_live_verification",
                    "platform_order_summary_live_verification",
                    "feedback_quality_live_refresh"
                )
                next_minimum_action = "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
            }
            next_commands = [ordered]@{
                refresh_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-refresh-current-status.ps1"
                read_status_refresh = "Get-Content -Raw -LiteralPath target\fb2-ai-center\status-refresh-current.json | ConvertFrom-Json"
                generate_context_pack_sample_request = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -PrintExportRequest -ExternalUserId 6fe5aa17-0403-427a-8e91-7f414beca35d -OutputPath target\fb2-ai-center\context-pack-sample-request-current.json"
                validate_context_pack_sample_set = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\context-pack-samples-validation-current.json"
                validate_exported_context_pack_sample_set = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir <fb2_repo>\target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\fb2-repo-context-pack-samples-validation-current.json"
                validate_context_projection_log = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-projection-log.ps1 -StatusPath target\fb2-ai-center\status-current.json -OutputPath target\fb2-ai-center\context-projection-log-validation-current.json"
                validate_current_state = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-current-state.ps1"
                validate_public_contract_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-public-contract-status.ps1 -OutputPath target\fb2-ai-center\public-contract-status-current.json"
                validate_server_deploy_status = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-main-server-deploy-status.ps1"
                validate_read_only_direct_read = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-readonly-summary.ps1 -SummaryPath target\fb2-ai-center\read-only-direct-read-current.json"
                validate_gap_action_board = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-gap-action-board.ps1"
                validate_evidence_freshness = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-evidence-freshness.ps1"
                validate_completion_matrix = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-completion-matrix.ps1"
                validate_handoff_prompt = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-handoff-prompt.ps1"
                validate_visible_answer_policy = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-answer-policy.ps1 -SummaryPath <DATA_ONLY_ACCEPTANCE_JSON>"
                validate_live_preflight_request = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-live-preflight-request.ps1 -StatusPath target\fb2-ai-center\status-current.json"
                validate_tokenless_continuation = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-tokenless-continuation.ps1 -OutputPath target\fb2-ai-center\tokenless-continuation-validation-current.json"
                no_write_direct_read = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD>"
                data_only_preflight = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD> -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>"
                visible_regression_requires_authorization = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD> -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>"
            }
            gap_action_board = [ordered]@{
                schema = "fb2.main_project.gap_action_board.v1"
                next_minimum_action = "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
                action_count = 3
                actions = @(
                    [ordered]@{
                        id = "FB2_AI_CENTER_TOKEN_live_permission_quality_refresh"
                        status = "blocked_by_external_secret"
                        owner = "fb2_project_and_shared"
                        evidence_needed = "service token"
                        command = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>"
                        notes = "no write"
                        can_run_without_secret = $false
                        requires_visible_group_write = $false
                        deferred_by_user = $false
                    },
                    [ordered]@{
                        id = "voice_final_evidence"
                        status = "deferred_by_user"
                        owner = "paused_by_user"
                        evidence_needed = "voice evidence"
                        command = ""
                        notes = "paused"
                        can_run_without_secret = $false
                        requires_visible_group_write = $false
                        deferred_by_user = $true
                    },
                    [ordered]@{
                        id = "full_final_acceptance_same_batch_voice_and_visible_chat"
                        status = "waiting_on_voice_and_authorized_visible_regression"
                        owner = "shared"
                        evidence_needed = "same batch final evidence"
                        command = "pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -AllowVisibleMessages"
                        notes = "requires explicit authorization"
                        can_run_without_secret = $false
                        requires_visible_group_write = $true
                        deferred_by_user = $false
                    }
                )
            }
        }
        $fixture | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $refreshPath -Encoding UTF8
        $validation = New-Fb2GapValidation -Refresh (Read-Fb2GapJson -Path $refreshPath) -SourcePath $refreshPath
        if (-not [bool]$validation.success) {
            $validation | ConvertTo-Json -Depth 8
            throw "SelfTest failed: gap validation fixture failed"
        }
        $badFixture = $fixture | ConvertTo-Json -Depth 8 | ConvertFrom-Json
        $badFixture.blocking_state.safe_to_continue_without_secret = @("public_contract_regression", "status_refresh_selftest")
        $badValidation = New-Fb2GapValidation -Refresh $badFixture -SourcePath "selftest-bad-missing-safe-list.json"
        if ([bool]$badValidation.success) {
            throw "SelfTest failed: missing blocking_state safe actions should fail"
        }
        $badCommandFixture = $fixture | ConvertTo-Json -Depth 8 | ConvertFrom-Json
        $badCommandFixture.next_commands.data_only_preflight = $badCommandFixture.next_commands.visible_regression_requires_authorization
        $badCommandValidation = New-Fb2GapValidation -Refresh $badCommandFixture -SourcePath "selftest-bad-visible-preflight.json"
        if ([bool]$badCommandValidation.success) {
            throw "SelfTest failed: data_only_preflight with visible write should fail"
        }
        $badOwnerFixture = $fixture | ConvertTo-Json -Depth 8 | ConvertFrom-Json
        $badOwnerFixture.owner_next_actions.fb2_project = "continue later"
        $badOwnerValidation = New-Fb2GapValidation -Refresh $badOwnerFixture -SourcePath "selftest-bad-owner-next-actions.json"
        if ([bool]$badOwnerValidation.success) {
            throw "SelfTest failed: vague owner_next_actions should fail"
        }
        "== SelfTest Summary =="
        "failed=0"
    } finally {
        if (Test-Path -LiteralPath $tempRoot) {
            Remove-Item -LiteralPath $tempRoot -Recurse -Force
        }
    }
}

if ($SelfTest) {
    Invoke-Fb2GapSelfTest
    exit 0
}

$root = Get-Fb2GapRepoRoot
if ([string]::IsNullOrWhiteSpace($RefreshPath)) {
    $RefreshPath = Join-Path $root "target\fb2-ai-center\status-refresh-current.json"
} else {
    $RefreshPath = Resolve-Fb2GapPath -Path $RefreshPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\gap-action-board-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2GapPath -Path $OutputPath -Root $root
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$refresh = Read-Fb2GapJson -Path $RefreshPath
$result = New-Fb2GapValidation -Refresh $refresh -SourcePath $RefreshPath
$json = $result | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $OutputPath -Value $json -Encoding UTF8
$json

if (-not [bool]$result.success) {
    exit 1
}
