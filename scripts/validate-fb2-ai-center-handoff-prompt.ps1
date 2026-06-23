#requires -Version 7.0

param(
    [string]$PromptPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2PromptValidationRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2PromptValidationPath {
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

function Add-Fb2PromptValidationCheck {
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

function Test-Fb2PromptValidationSecretSafe {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) {
        return $true
    }
    if ($Text -match '(?i)FB2_AI_CENTER_TOKEN\s*=\s*["''][^<]') {
        return $false
    }
    if ($Text -match '(?i)-Fb2(AiCenter)?Token\s+(?!<FB2_AI_CENTER_TOKEN>)[^\s`]+') {
        return $false
    }
    if ($Text -match '(?i)-Fb2Password\s+(?!<FB2_PASSWORD>)[^\s`]+') {
        return $false
    }
    if ($Text -match '(?i)(bearer|token|password|secret)[=:]\s*(?!<)[A-Za-z0-9_\-\.]{12,}') {
        return $false
    }
    return $true
}

function New-Fb2PromptValidation {
    param(
        [string]$Content,
        [string]$SourcePath
    )

    $checks = [System.Collections.ArrayList]::new()
    $lineCount = if ([string]::IsNullOrEmpty($Content)) { 0 } else { @($Content -split "(`r`n|`n|`r)").Count }
    Add-Fb2PromptValidationCheck $checks "prompt exists" (-not [string]::IsNullOrWhiteSpace($Content))
    Add-Fb2PromptValidationCheck $checks "prompt has enough lines" ($lineCount -ge 30) "lines=$lineCount"
    Add-Fb2PromptValidationCheck $checks "title present" ($Content -match '# fb2 AI Center 下一轮执行提示')
    Add-Fb2PromptValidationCheck $checks "status refresh schema present" ($Content -match 'fb2\.main_project\.status_refresh\.v1')
    Add-Fb2PromptValidationCheck $checks "completion matrix schema present" ($Content -match 'fb2\.main_project\.completion_matrix\.v1')

    foreach ($section in @(
            "## 当前闸门",
            "## Owner 下一步",
            "## 可执行命令",
            "## 阻塞与边界",
            "## 缺口行动板",
            "## 证据新鲜度",
            "## 完成矩阵",
            "## 接手规则"
        )) {
        Add-Fb2PromptValidationCheck $checks "section $section" ($Content.Contains($section))
    }

    foreach ($field in @(
            "data_goal_complete",
            "full_final_complete",
            "token_present",
            "voice_deferred_by_user",
            "next_minimum_action"
        )) {
        Add-Fb2PromptValidationCheck $checks "gate $field" ($Content -match [regex]::Escape($field))
    }

    $requiredCommands = @(
        "refresh_status",
        "read_status_refresh",
        "generate_context_pack_sample_request",
        "validate_context_pack_sample_set",
        "validate_exported_context_pack_sample_set",
        "validate_current_state",
        "validate_server_deploy_status",
        "validate_read_only_direct_read",
        "validate_gap_action_board",
        "validate_evidence_freshness",
        "validate_completion_matrix",
        "validate_handoff_prompt",
        "validate_visible_answer_policy",
        "no_write_direct_read",
        "data_only_preflight",
        "visible_regression_requires_authorization"
    )
    foreach ($command in $requiredCommands) {
        $inlineCommand = '`{0}`' -f $command
        Add-Fb2PromptValidationCheck $checks "command $command" ($Content -match [regex]::Escape($inlineCommand))
    }

    Add-Fb2PromptValidationCheck $checks "token placeholder present" ($Content -match '<FB2_AI_CENTER_TOKEN>')
    Add-Fb2PromptValidationCheck $checks "password placeholder present" ($Content -match '<FB2_PASSWORD>')
    Add-Fb2PromptValidationCheck $checks "context pack sample request prints export request" ($Content -match 'generate_context_pack_sample_request.+PrintExportRequest')
    Add-Fb2PromptValidationCheck $checks "context pack sample set validates sample set" ($Content -match 'validate_context_pack_sample_set.+ValidateSampleSet')
    Add-Fb2PromptValidationCheck $checks "exported context pack sample set validates fb2 repo samples" ($Content -match 'validate_exported_context_pack_sample_set.+ValidateSampleSet.+fb2-repo-context-pack-samples-validation-current\.json')
    Add-Fb2PromptValidationCheck $checks "data-only preflight is no visible write" ($Content -match 'data_only_preflight.+DataOnlyAcceptance.+PreflightOnly')
    Add-Fb2PromptValidationCheck $checks "visible regression is explicit" ($Content -match 'visible_regression_requires_authorization.+AllowVisibleMessages')
    Add-Fb2PromptValidationCheck $checks "voice pause is explicit" ($Content -match 'ASR/TTS final evidence.*暂停|ASR/TTS.*paused|voice_final_evidence')

    $requiredSafeWithoutSecret = @(
        "public_contract_regression",
        "status_refresh_selftest",
        "offline_context_pack_sample_validation",
        "handoff_documentation"
    )
    foreach ($item in $requiredSafeWithoutSecret) {
        Add-Fb2PromptValidationCheck $checks "safe without secret $item" (
            ($Content -match 'safe_to_continue_without_secret') -and
            ($Content -match [regex]::Escape($item))
        )
    }

    $requiredSecretGates = @(
        "live_context_pack_permission_quality_refresh",
        "current_user_order_live_verification",
        "platform_order_summary_live_verification",
        "feedback_quality_live_refresh"
    )
    foreach ($item in $requiredSecretGates) {
        Add-Fb2PromptValidationCheck $checks "requires secret $item" (
            ($Content -match 'requires_secret') -and
            ($Content -match [regex]::Escape($item))
        )
    }

    $forbiddenPatterns = @(
        'Add-Fb2PromptLine',
        '\$\(\[bool\]',
        '\$SourcePath',
        'secret-real-value',
        'secret-real-password',
        'System\.Object\[\]'
    )
    foreach ($pattern in $forbiddenPatterns) {
        Add-Fb2PromptValidationCheck $checks "forbidden fragment $pattern" (-not ($Content -match $pattern))
    }
    Add-Fb2PromptValidationCheck $checks "secret safe" (Test-Fb2PromptValidationSecretSafe -Text $Content)

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.handoff_prompt_validation.v1"
        source_prompt = $SourcePath
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
    }
}

function Invoke-Fb2PromptValidationSelfTest {
    $good = @'
# fb2 AI Center 下一轮执行提示

来源 refresh summary: `target\fb2-ai-center\status-refresh-current.json`
schema: `fb2.main_project.status_refresh.v1` / matrix: `fb2.main_project.completion_matrix.v1`

## 当前闸门
- data_goal_complete: `True`
- full_final_complete: `False`
- token_present: `False`
- voice_deferred_by_user: `True`
- next_minimum_action: `set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly`
- totals: complete `13` / deferred `1` / incomplete `0` / total `14`

## Owner 下一步
- main_project: `keep_contract_and_status_regressions_green_until_FB2_AI_CENTER_TOKEN_is_available`
- fb2_project: `provide_FB2_AI_CENTER_TOKEN_or_export_equivalent_live_Context_Pack_permission_quality_evidence`
- shared: `run_DataOnlyAcceptance_PreflightOnly_with_token_then_refresh_status_refresh_current_json`

## 可执行命令
- `refresh_status`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-refresh-current-status.ps1`
- `read_status_refresh`: `Get-Content -Raw -LiteralPath target\fb2-ai-center\status-refresh-current.json | ConvertFrom-Json`
- `generate_context_pack_sample_request`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -PrintExportRequest -ExternalUserId <fb2_user_uuid_with_orders> -OutputPath target\fb2-ai-center\context-pack-sample-request-current.json`
- `validate_context_pack_sample_set`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\context-pack-samples-validation-current.json`
- `validate_exported_context_pack_sample_set`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir <fb2_repo>\target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\fb2-repo-context-pack-samples-validation-current.json`
- `validate_current_state`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-current-state.ps1`
- `validate_server_deploy_status`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-main-server-deploy-status.ps1`
- `validate_read_only_direct_read`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-readonly-summary.ps1 -SummaryPath target\fb2-ai-center\read-only-direct-read-current.json`
- `validate_gap_action_board`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-gap-action-board.ps1`
- `validate_evidence_freshness`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-evidence-freshness.ps1`
- `validate_completion_matrix`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-completion-matrix.ps1`
- `validate_handoff_prompt`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-handoff-prompt.ps1`
- `validate_visible_answer_policy`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-answer-policy.ps1 -SummaryPath <DATA_ONLY_ACCEPTANCE_JSON>`
- `no_write_direct_read`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD>`
- `data_only_preflight`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD> -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>`
- `visible_regression_requires_authorization`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD> -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>`

## 阻塞与边界
- external_secret: `FB2_AI_CENTER_TOKEN`
- blocked_by_external_secret: `True`
- safe_to_continue_without_secret: `public_contract_regression, status_refresh_selftest, offline_context_pack_sample_validation, handoff_documentation`
- requires_secret: `live_context_pack_permission_quality_refresh, current_user_order_live_verification, platform_order_summary_live_verification, feedback_quality_live_refresh`

## 缺口行动板
- gap_schema: `fb2.main_project.gap_action_board.v1`
- action_count: `4`
- gap voice_final_evidence: status=deferred_by_user; owner=paused_by_user; evidence=real device ASR/TTS final-ready evidence JSON; command=; notes=ASR/TTS final evidence 仍按用户要求暂停。

## 证据新鲜度
- freshness_schema: `fb2.main_project.evidence_freshness.v1`
| artifact | source | age_minutes | path |
|---|---|---:|---|
| status | current_output_dir | 0 | target\fb2-ai-center\status-current.json |

## 完成矩阵
| group | owner | id | status | evidence | missing |
|---|---|---|---|---|---|
| voice_deferred_by_user | paused_by_user | voice_final_evidence | deferred | voice_final_evidence_path_present=False | ASR/TTS is intentionally deferred by user |

## 接手规则
- 先运行 `refresh_status`，再读取 `status-refresh-current.json`。
'@
    $badScript = $good + "`nAdd-Fb2PromptLine -Lines `$lines"
    $badSecret = $good -replace '<FB2_AI_CENTER_TOKEN>', 'real-secret-token-1234567890'
    $badBoundary = $good -replace 'public_contract_regression, status_refresh_selftest, offline_context_pack_sample_validation, handoff_documentation', 'public_contract_regression, status_refresh_selftest'

    $failed = 0
    $goodResult = New-Fb2PromptValidation -Content $good -SourcePath "selftest-good.md"
    if (-not [bool]$goodResult.success) {
        $goodResult | ConvertTo-Json -Depth 8
        $failed++
    }
    $badScriptResult = New-Fb2PromptValidation -Content $badScript -SourcePath "selftest-script.md"
    if ([bool]$badScriptResult.success) { $failed++ }
    $badSecretResult = New-Fb2PromptValidation -Content $badSecret -SourcePath "selftest-secret.md"
    if ([bool]$badSecretResult.success) { $failed++ }
    $badBoundaryResult = New-Fb2PromptValidation -Content $badBoundary -SourcePath "selftest-boundary.md"
    if ([bool]$badBoundaryResult.success) { $failed++ }

    Write-Output "== SelfTest Summary =="
    Write-Output "failed=$failed"
    if ($failed -gt 0) {
        exit 1
    }
}

if ($SelfTest) {
    Invoke-Fb2PromptValidationSelfTest
    exit 0
}

$root = Get-Fb2PromptValidationRepoRoot
if ([string]::IsNullOrWhiteSpace($PromptPath)) {
    $PromptPath = Join-Path $root "target\fb2-ai-center\handoff-prompt-current.md"
} else {
    $PromptPath = Resolve-Fb2PromptValidationPath -Path $PromptPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\handoff-prompt-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2PromptValidationPath -Path $OutputPath -Root $root
}

if (-not (Test-Path -LiteralPath $PromptPath)) {
    throw "Handoff prompt not found: $PromptPath. Run scripts\fb2-ai-center-refresh-current-status.ps1 first."
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$content = Get-Content -LiteralPath $PromptPath -Raw
$result = New-Fb2PromptValidation -Content $content -SourcePath $PromptPath
$json = $result | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $OutputPath -Value $json -Encoding UTF8
$json

if (-not [bool]$result.success) {
    exit 1
}
