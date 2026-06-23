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
        -Name "validate_evidence_freshness" `
        -ScriptPath (Join-Path $PSScriptRoot "validate-fb2-ai-center-evidence-freshness.ps1") `
        -Arguments @("-RefreshPath", $RefreshPath) `
        -ExpectedOutputPath (Join-Path $targetDir "evidence-freshness-validation-current.json")))
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

    $refresh = Read-Fb2CurrentJsonOrNull -Path $RefreshPath
    $failedSteps = @($steps | Where-Object { -not [bool]$_.success -or -not [bool]$_.output_secret_safe })
    $blocking = Get-Fb2CurrentProperty $refresh "blocking_state"
    $completion = Get-Fb2CurrentProperty $refresh "completion_matrix"
    $gates = Get-Fb2CurrentProperty $completion "gates"
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
        [ordered]@{ name = "evidence_freshness"; script = "validate-fb2-ai-center-evidence-freshness.ps1" },
        [ordered]@{ name = "gap_action_board"; script = "validate-fb2-ai-center-gap-action-board.ps1" },
        [ordered]@{ name = "completion_matrix"; script = "validate-fb2-ai-center-completion-matrix.ps1" },
        [ordered]@{ name = "handoff_prompt"; script = "validate-fb2-ai-center-handoff-prompt.ps1" }
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
