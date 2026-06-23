#requires -Version 7.0

param(
    [string]$RefreshPath = "",
    [string]$OutputPath = "",
    [switch]$SkipRefresh,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2CurrentRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2CurrentPath {
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

function Get-Fb2CurrentProperty {
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

function Read-Fb2CurrentJsonOrNull {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return $null
    }
    try {
        Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    } catch {
        return $null
    }
}

function Test-Fb2CurrentSecretSafe {
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

function Invoke-Fb2CurrentPwsh {
    param(
        [string]$Name,
        [string]$ScriptPath,
        [string[]]$Arguments = @(),
        [string]$ExpectedOutputPath = ""
    )

    $command = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $ScriptPath) + @($Arguments)
    $output = & pwsh @command 2>&1
    $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
    $outputExists = [string]::IsNullOrWhiteSpace($ExpectedOutputPath) -or (Test-Path -LiteralPath $ExpectedOutputPath)
    $result = Read-Fb2CurrentJsonOrNull -Path $ExpectedOutputPath
    $outputParseable = [string]::IsNullOrWhiteSpace($ExpectedOutputPath) -or ($null -ne $result)
    $jsonSuccess = if ($null -eq $result) { $null } else { Get-Fb2CurrentProperty $result "success" $null }
    $success = ($exitCode -eq 0) -and $outputExists -and $outputParseable -and ($null -eq $jsonSuccess -or [bool]$jsonSuccess)
    [ordered]@{
        name = $Name
        exit_code = $exitCode
        success = [bool]$success
        output_path = $ExpectedOutputPath
        output_exists = [bool]$outputExists
        output_parseable = [bool]$outputParseable
        json_success = $jsonSuccess
        output_secret_safe = Test-Fb2CurrentSecretSafe -Text (@($output) -join "`n")
    }
}

function New-Fb2CurrentInlineStep {
    param(
        [string]$Name,
        [bool]$Success,
        [string]$OutputPath = "",
        [object]$JsonSuccess = $null,
        [string]$Details = ""
    )

    [ordered]@{
        name = $Name
        exit_code = 0
        success = [bool]$Success
        output_path = $OutputPath
        output_exists = [string]::IsNullOrWhiteSpace($OutputPath) -or (Test-Path -LiteralPath $OutputPath)
        output_parseable = $true
        json_success = $JsonSuccess
        output_secret_safe = Test-Fb2CurrentSecretSafe -Text $Details
    }
}

function Test-Fb2CurrentExportedSampleState {
    param([object]$State)

    if ($null -eq $State) {
        return $false
    }
    $enabled = [bool](Get-Fb2CurrentProperty $State "enabled" $false)
    $attempted = [bool](Get-Fb2CurrentProperty $State "attempted" $false)
    $complete = [bool](Get-Fb2CurrentProperty $State "complete" $false)
    $skippedReason = [string](Get-Fb2CurrentProperty $State "skipped_reason" "")
    if (-not $enabled) {
        return $true
    }
    if ($attempted) {
        return $complete
    }
    return -not [string]::IsNullOrWhiteSpace($skippedReason)
}

function New-Fb2CurrentStateValidation {
    param(
        [string]$RefreshPath,
        [string]$OutputPath,
        [bool]$SkipRefresh
    )

    $root = Get-Fb2CurrentRepoRoot
    $targetDir = Join-Path $root "target\fb2-ai-center"
    New-Item -ItemType Directory -Force -Path $targetDir | Out-Null

    if ([string]::IsNullOrWhiteSpace($RefreshPath)) {
        $RefreshPath = Join-Path $targetDir "status-refresh-current.json"
    } else {
        $RefreshPath = Resolve-Fb2CurrentPath -Path $RefreshPath -Root $root
    }
    if ([string]::IsNullOrWhiteSpace($OutputPath)) {
        $OutputPath = Join-Path $targetDir "current-state-validation-current.json"
    } else {
        $OutputPath = Resolve-Fb2CurrentPath -Path $OutputPath -Root $root
    }

    $steps = [System.Collections.ArrayList]::new()
    if (-not $SkipRefresh) {
        [void]$steps.Add((Invoke-Fb2CurrentPwsh `
            -Name "refresh_status" `
            -ScriptPath (Join-Path $PSScriptRoot "fb2-ai-center-refresh-current-status.ps1") `
            -Arguments @("-RefreshSummaryPath", $RefreshPath) `
            -ExpectedOutputPath $RefreshPath))
    }

    [void]$steps.Add((Invoke-Fb2CurrentPwsh `
        -Name "validate_public_contract_status" `
        -ScriptPath (Join-Path $PSScriptRoot "fb2-public-contract-status.ps1") `
        -Arguments @("-OutputPath", (Join-Path $targetDir "public-contract-status-current.json")) `
        -ExpectedOutputPath (Join-Path $targetDir "public-contract-status-current.json")))

    [void]$steps.Add((Invoke-Fb2CurrentPwsh `
        -Name "validate_server_deploy_status" `
        -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-main-server-deploy-status.ps1") `
        -Arguments @("-OutputPath", (Join-Path $targetDir "server-deploy-status-current.json")) `
        -ExpectedOutputPath (Join-Path $targetDir "server-deploy-status-current.json")))

    $refreshForOptionalSteps = Read-Fb2CurrentJsonOrNull -Path $RefreshPath
    $exportedSampleState = Get-Fb2CurrentProperty $refreshForOptionalSteps "exported_context_pack_sample_set_validation"
    if ($null -ne $exportedSampleState) {
        [void]$steps.Add((New-Fb2CurrentInlineStep `
            -Name "validate_exported_context_pack_sample_set" `
            -Success (Test-Fb2CurrentExportedSampleState -State $exportedSampleState) `
            -OutputPath ([string](Get-Fb2CurrentProperty $exportedSampleState "output_path" "")) `
            -JsonSuccess (Get-Fb2CurrentProperty $exportedSampleState "success" $null) `
            -Details ($exportedSampleState | ConvertTo-Json -Depth 6)))
    }
    $filesForOptionalSteps = Get-Fb2CurrentProperty $refreshForOptionalSteps "files"
    $statusPathForOptionalSteps = [string](Get-Fb2CurrentProperty $filesForOptionalSteps "status" "")
    $statusForOptionalSteps = Read-Fb2CurrentJsonOrNull -Path $statusPathForOptionalSteps
    $latestReadOnly = Get-Fb2CurrentProperty $statusForOptionalSteps "latest_read_only_direct_read"
    $latestReadOnlyPath = [string](Get-Fb2CurrentProperty $latestReadOnly "path" "")
    if (-not [string]::IsNullOrWhiteSpace($latestReadOnlyPath) -and (Test-Path -LiteralPath $latestReadOnlyPath)) {
        [void]$steps.Add((Invoke-Fb2CurrentPwsh `
            -Name "validate_read_only_direct_read" `
            -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-visible-readonly-summary.ps1") `
            -Arguments @("-SummaryPath", $latestReadOnlyPath, "-OutputPath", (Join-Path $targetDir "visible-readonly-summary-validation-current.json")) `
            -ExpectedOutputPath (Join-Path $targetDir "visible-readonly-summary-validation-current.json")))
    }
    $latestDataOnly = Get-Fb2CurrentProperty $statusForOptionalSteps "latest_data_only_acceptance"
    $latestDataOnlyPath = [string](Get-Fb2CurrentProperty $latestDataOnly "path" "")
    $visibleAnswerPolicyValidationPath = Join-Path $targetDir "visible-answer-policy-validation-current.json"
    if (-not [string]::IsNullOrWhiteSpace($latestDataOnlyPath) -and (Test-Path -LiteralPath $latestDataOnlyPath)) {
        [void]$steps.Add((Invoke-Fb2CurrentPwsh `
            -Name "validate_visible_answer_policy" `
            -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-visible-answer-policy.ps1") `
            -Arguments @("-SummaryPath", $latestDataOnlyPath, "-OutputPath", $visibleAnswerPolicyValidationPath) `
            -ExpectedOutputPath $visibleAnswerPolicyValidationPath))
    } else {
        [void]$steps.Add((New-Fb2CurrentInlineStep `
            -Name "validate_visible_answer_policy" `
            -Success $false `
            -OutputPath "" `
            -JsonSuccess $false `
            -Details "missing_latest_data_only_acceptance_summary"))
    }

    $livePreflightValidationPath = Join-Path $targetDir "live-preflight-request-validation-current.json"
    if (-not [string]::IsNullOrWhiteSpace($statusPathForOptionalSteps) -and (Test-Path -LiteralPath $statusPathForOptionalSteps)) {
        [void]$steps.Add((Invoke-Fb2CurrentPwsh `
            -Name "validate_live_preflight_request" `
            -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-live-preflight-request.ps1") `
            -Arguments @("-StatusPath", $statusPathForOptionalSteps, "-OutputPath", $livePreflightValidationPath) `
            -ExpectedOutputPath $livePreflightValidationPath))
    } else {
        [void]$steps.Add((New-Fb2CurrentInlineStep `
            -Name "validate_live_preflight_request" `
            -Success $false `
            -OutputPath "" `
            -JsonSuccess $false `
            -Details "missing_status_for_live_preflight_request"))
    }

    $contextProjectionValidationPath = Join-Path $targetDir "context-projection-log-validation-current.json"
    if (-not [string]::IsNullOrWhiteSpace($statusPathForOptionalSteps) -and (Test-Path -LiteralPath $statusPathForOptionalSteps)) {
        [void]$steps.Add((Invoke-Fb2CurrentPwsh `
            -Name "validate_context_projection_log" `
            -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-context-projection-log.ps1") `
            -Arguments @("-StatusPath", $statusPathForOptionalSteps, "-OutputPath", $contextProjectionValidationPath) `
            -ExpectedOutputPath $contextProjectionValidationPath))
    } else {
        [void]$steps.Add((New-Fb2CurrentInlineStep `
            -Name "validate_context_projection_log" `
            -Success $false `
            -OutputPath "" `
            -JsonSuccess $false `
            -Details "missing_status_for_context_projection_log"))
    }

    $userScenarioValidationPath = Join-Path $targetDir "user-scenario-audit-validation-current.json"
    if (-not [string]::IsNullOrWhiteSpace($statusPathForOptionalSteps) -and (Test-Path -LiteralPath $statusPathForOptionalSteps)) {
        [void]$steps.Add((Invoke-Fb2CurrentPwsh `
            -Name "validate_user_scenario_audit" `
            -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-user-scenario-audit.ps1") `
            -Arguments @("-StatusPath", $statusPathForOptionalSteps, "-OutputPath", $userScenarioValidationPath) `
            -ExpectedOutputPath $userScenarioValidationPath))
    } else {
        [void]$steps.Add((New-Fb2CurrentInlineStep `
            -Name "validate_user_scenario_audit" `
            -Success $false `
            -OutputPath "" `
            -JsonSuccess $false `
            -Details "missing_status_for_user_scenario_audit"))
    }

    [void]$steps.Add((Invoke-Fb2CurrentPwsh `
        -Name "validate_evidence_freshness" `
        -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-ai-center-evidence-freshness.ps1") `
        -Arguments @("-RefreshPath", $RefreshPath) `
        -ExpectedOutputPath (Join-Path $targetDir "evidence-freshness-validation-current.json")))
    [void]$steps.Add((Invoke-Fb2CurrentPwsh `
        -Name "validate_evidence_privacy" `
        -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-evidence-privacy.ps1") `
        -Arguments @("-RefreshPath", $RefreshPath) `
        -ExpectedOutputPath (Join-Path $targetDir "evidence-privacy-validation-current.json")))
    [void]$steps.Add((Invoke-Fb2CurrentPwsh `
        -Name "validate_gap_action_board" `
        -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-ai-center-gap-action-board.ps1") `
        -Arguments @("-RefreshPath", $RefreshPath) `
        -ExpectedOutputPath (Join-Path $targetDir "gap-action-board-validation-current.json")))
    [void]$steps.Add((Invoke-Fb2CurrentPwsh `
        -Name "validate_completion_matrix" `
        -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-ai-center-completion-matrix.ps1") `
        -Arguments @("-RefreshPath", $RefreshPath) `
        -ExpectedOutputPath (Join-Path $targetDir "completion-matrix-validation-current.json")))
    [void]$steps.Add((Invoke-Fb2CurrentPwsh `
        -Name "validate_handoff_prompt" `
        -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-ai-center-handoff-prompt.ps1") `
        -Arguments @("-RefreshPath", $RefreshPath) `
        -ExpectedOutputPath (Join-Path $targetDir "handoff-prompt-validation-current.json")))
    $tokenlessContinuationValidationPath = Join-Path $targetDir "tokenless-continuation-validation-current.json"
    [void]$steps.Add((Invoke-Fb2CurrentPwsh `
        -Name "validate_tokenless_continuation" `
        -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-tokenless-continuation.ps1") `
        -Arguments @("-RefreshPath", $RefreshPath, "-OutputPath", $tokenlessContinuationValidationPath) `
        -ExpectedOutputPath $tokenlessContinuationValidationPath))

    $refresh = Read-Fb2CurrentJsonOrNull -Path $RefreshPath
    $failedSteps = @($steps | Where-Object { -not [bool]$_.success -or -not [bool]$_.output_secret_safe })
    $blocking = Get-Fb2CurrentProperty $refresh "blocking_state"
    $completion = Get-Fb2CurrentProperty $refresh "completion_matrix"
    $gates = Get-Fb2CurrentProperty $completion "gates"
    $exportedSampleValidation = Get-Fb2CurrentProperty $refresh "exported_context_pack_sample_set_validation"
    $publicContractStatus = Read-Fb2CurrentJsonOrNull -Path (Join-Path $targetDir "public-contract-status-current.json")
    $visibleAnswerPolicyValidation = Read-Fb2CurrentJsonOrNull -Path $visibleAnswerPolicyValidationPath
    $livePreflightRequestValidation = Read-Fb2CurrentJsonOrNull -Path $livePreflightValidationPath
    $tokenlessContinuationValidation = Read-Fb2CurrentJsonOrNull -Path $tokenlessContinuationValidationPath
    $contextProjectionLogValidation = Read-Fb2CurrentJsonOrNull -Path $contextProjectionValidationPath
    $userScenarioAuditValidation = Read-Fb2CurrentJsonOrNull -Path $userScenarioValidationPath
    $result = [ordered]@{
        schema = "fb2.main_project.current_state_validation.v1"
        generated_at_utc = ([datetime]::UtcNow).ToString("o")
        refresh_path = $RefreshPath
        success = (@($failedSteps).Count -eq 0 -and $null -ne $refresh)
        step_count = @($steps).Count
        failed_count = @($failedSteps).Count
        failed = @($failedSteps)
        steps = @($steps)
        data_goal_complete = [bool](Get-Fb2CurrentProperty $refresh "data_goal_complete" $false)
        full_final_complete = [bool](Get-Fb2CurrentProperty $refresh "full_final_complete" $false)
        token_present = [bool](Get-Fb2CurrentProperty $refresh "token_present" $false)
        voice_deferred_by_user = [bool](Get-Fb2CurrentProperty $gates "voice_deferred_by_user" $false)
        next_minimum_action = [string](Get-Fb2CurrentProperty $refresh "next_minimum_action" "")
        blocked_by_external_secret = [bool](Get-Fb2CurrentProperty $blocking "blocked_by_external_secret" $false)
        public_contract_status = $publicContractStatus
        exported_context_pack_sample_set_validation = $exportedSampleValidation
        visible_answer_policy_validation = $visibleAnswerPolicyValidation
        live_preflight_request_validation = $livePreflightRequestValidation
        tokenless_continuation_validation = $tokenlessContinuationValidation
        context_projection_log_validation = $contextProjectionLogValidation
        user_scenario_audit_validation = $userScenarioAuditValidation
        safe_to_continue_without_secret = @((Get-Fb2CurrentProperty $blocking "safe_to_continue_without_secret" @()))
        requires_secret = @((Get-Fb2CurrentProperty $blocking "requires_secret" @()))
        note = "This gate refreshes and validates current machine evidence only; protected live fb2 data still requires FB2_AI_CENTER_TOKEN."
    }

    $parent = Split-Path -Parent $OutputPath
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $json = $result | ConvertTo-Json -Depth 8
    Set-Content -LiteralPath $OutputPath -Value $json -Encoding UTF8
    $json

    if (-not [bool]$result.success) {
        exit 1
    }
}

function Invoke-Fb2CurrentStateSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-current-state-selftest-" + [guid]::NewGuid().ToString("N"))
    $steps = @(
        [ordered]@{ name = "refresh_status"; script = "fb2-ai-center-refresh-current-status.ps1" },
        [ordered]@{ name = "public_contract_status"; script = "fb2-public-contract-status.ps1" },
        [ordered]@{ name = "server_deploy_status"; script = "validate-fb2-main-server-deploy-status.ps1" },
        [ordered]@{ name = "visible_readonly_summary"; script = "validate-fb2-visible-readonly-summary.ps1" },
        [ordered]@{ name = "visible_answer_policy"; script = "validate-fb2-visible-answer-policy.ps1" },
        [ordered]@{ name = "live_preflight_request"; script = "validate-fb2-live-preflight-request.ps1" },
        [ordered]@{ name = "context_projection_log"; script = "validate-fb2-context-projection-log.ps1" },
        [ordered]@{ name = "user_scenario_audit"; script = "validate-fb2-user-scenario-audit.ps1" },
        [ordered]@{ name = "evidence_freshness"; script = "validate-fb2-ai-center-evidence-freshness.ps1" },
        [ordered]@{ name = "evidence_privacy"; script = "validate-fb2-evidence-privacy.ps1" },
        [ordered]@{ name = "gap_action_board"; script = "validate-fb2-ai-center-gap-action-board.ps1" },
        [ordered]@{ name = "completion_matrix"; script = "validate-fb2-ai-center-completion-matrix.ps1" },
        [ordered]@{ name = "handoff_prompt"; script = "validate-fb2-ai-center-handoff-prompt.ps1" },
        [ordered]@{ name = "tokenless_continuation"; script = "validate-fb2-tokenless-continuation.ps1" }
    )
    try {
        New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
        $failed = 0
        foreach ($step in $steps) {
            $scriptPath = Join-Path $PSScriptRoot ([string]$step.script)
            $output = & pwsh -NoProfile -ExecutionPolicy Bypass -File $scriptPath -SelfTest 2>&1
            $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
            if ($exitCode -ne 0 -or -not (Test-Fb2CurrentSecretSafe -Text (@($output) -join "`n"))) {
                $failed++
            }
        }

        $noOutputScript = Join-Path $tempRoot "no-output.ps1"
        Set-Content -LiteralPath $noOutputScript -Value "exit 0" -Encoding UTF8
        $missingOutput = Invoke-Fb2CurrentPwsh `
            -Name "missing_output_fixture" `
            -ScriptPath $noOutputScript `
            -ExpectedOutputPath (Join-Path $tempRoot "missing-output.json")
        if ([bool]$missingOutput.success -or [bool]$missingOutput.output_exists -or [bool]$missingOutput.output_parseable) {
            $failed++
        }

        $invalidOutputScript = Join-Path $tempRoot "invalid-output.ps1"
        $invalidOutputPath = Join-Path $tempRoot "invalid-output.json"
        Set-Content -LiteralPath $invalidOutputScript -Value "Set-Content -LiteralPath '$invalidOutputPath' -Value 'not-json' -Encoding UTF8; exit 0" -Encoding UTF8
        $invalidOutput = Invoke-Fb2CurrentPwsh `
            -Name "invalid_output_fixture" `
            -ScriptPath $invalidOutputScript `
            -ExpectedOutputPath $invalidOutputPath
        if ([bool]$invalidOutput.success -or -not [bool]$invalidOutput.output_exists -or [bool]$invalidOutput.output_parseable) {
            $failed++
        }

        if (Test-Fb2CurrentExportedSampleState -State $null) {
            $failed++
        }
        $skippedExportedSamples = [pscustomobject]@{
            enabled = $true
            attempted = $false
            skipped_reason = "samples_dir_missing"
            complete = $false
        }
        if (-not (Test-Fb2CurrentExportedSampleState -State $skippedExportedSamples)) {
            $failed++
        }
        $failedExportedSamples = [pscustomobject]@{
            enabled = $true
            attempted = $true
            skipped_reason = ""
            complete = $false
        }
        if (Test-Fb2CurrentExportedSampleState -State $failedExportedSamples) {
            $failed++
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
    Invoke-Fb2CurrentStateSelfTest
    exit 0
}

New-Fb2CurrentStateValidation -RefreshPath $RefreshPath -OutputPath $OutputPath -SkipRefresh:([bool]$SkipRefresh)
