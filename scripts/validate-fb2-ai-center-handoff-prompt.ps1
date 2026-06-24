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
            "## 计划能力 / 非生产边界",
            "## fb2 导出样本",
            "## 可执行命令",
            "## 阻塞与边界",
            "## 缺口行动板",
            "## 证据新鲜度",
            "## 完成矩阵",
            "## 接手规则"
        )) {
        Add-Fb2PromptValidationCheck $checks "section $section" ($Content.Contains($section))
    }
    Add-Fb2PromptValidationCheck $checks "section ## 线上主项目" ($Content.Contains("## 线上主项目"))
    Add-Fb2PromptValidationCheck $checks "exported sample table has business source column" ($Content -match '\|\s*scenario\s*\|\s*audit\s*\|\s*sources\s*\|\s*business\s*\|\s*quality_history\s*\|\s*sha256\s*\|')
    $exportedSampleRows = @($Content -split "(`r`n|`n|`r)" | Where-Object { $_ -match '^\|\s*[^|]+_context_pack\s*\|' })
    $reviewSummaryBusinessCells = @()
    $reviewSummaryMissingQualityCells = @()
    foreach ($row in $exportedSampleRows) {
        $cells = @($row -split '\|' | ForEach-Object { $_.Trim() })
        if ($cells.Count -lt 7) {
            continue
        }
        $businessCell = [string]$cells[4]
        $qualityCell = [string]$cells[5]
        if ($businessCell -match 'opinion_result_review_summary') {
            $reviewSummaryBusinessCells += $row
        }
        if (($row -match 'opinion_result_review_summary') -and ($qualityCell -notmatch 'opinion_result_review_summary')) {
            $reviewSummaryMissingQualityCells += $row
        }
    }
    Add-Fb2PromptValidationCheck $checks "review summary is not shown as exported sample business source" (@($reviewSummaryBusinessCells).Count -eq 0) (@($reviewSummaryBusinessCells) -join "`n")
    Add-Fb2PromptValidationCheck $checks "review summary is shown as exported sample quality history source" (@($reviewSummaryMissingQualityCells).Count -eq 0) (@($reviewSummaryMissingQualityCells) -join "`n")
    Add-Fb2PromptValidationCheck $checks "planned vector contract visible" ($Content -match 'fb2_p4_vector_contract_v1')
    Add-Fb2PromptValidationCheck $checks "planned vector report visible" ($Content -match 'fb2_p4_vector_readiness_plan_v1')
    Add-Fb2PromptValidationCheck $checks "planned embedding dry-run report visible" ($Content -match 'fb2_p4_embedding_build_dry_run_v1')
    Add-Fb2PromptValidationCheck $checks "planned vector status visible" ($Content -match 'contract_design_committed_embedding_not_started')
    Add-Fb2PromptValidationCheck $checks "planned vector production boundary visible" ($Content -match 'production_grounding')
    Add-Fb2PromptValidationCheck $checks "planned vector non-blocking boundary visible" ($Content -match 'blocks_data_goal')
    Add-Fb2PromptValidationCheck $checks "planned vector answer-time boundary visible" ($Content -match 'answer_time_vector_candidates_enabled')
    Add-Fb2PromptValidationCheck $checks "planned embedding dry-run no-write boundary visible" ($Content -match 'dry_run_available_no_writes|writes_vector_store=false|candidate_rows_require_live_hydration')

    foreach ($field in @(
            "data_goal_complete",
            "full_final_complete",
            "token_present",
            "protected_live_preflight_satisfied",
            "answer_source_validation_ready",
            "voice_deferred_by_user",
            "next_minimum_action"
        )) {
        Add-Fb2PromptValidationCheck $checks "gate $field" ($Content -match [regex]::Escape($field))
    }

    foreach ($field in @(
            "main_base",
            "health",
            "versionName",
            "deployed_git_sha",
            "latest_runtime_sha",
            "deployed_contains_latest_runtime_sha",
            "server_deploy_ready"
        )) {
        Add-Fb2PromptValidationCheck $checks "server deploy field $field" ($Content -match [regex]::Escape($field))
    }
    Add-Fb2PromptValidationCheck $checks "server deploy health ok visible" ($Content -match 'health:\s+`OK`')
    Add-Fb2PromptValidationCheck $checks "server deploy version visible" ($Content -match 'versionName:\s+`[^`]+`')
    Add-Fb2PromptValidationCheck $checks "server deploy sha visible" ($Content -match 'deployed_git_sha:\s+`[0-9a-f]{7,40}`')
    Add-Fb2PromptValidationCheck $checks "latest runtime sha visible" ($Content -match 'latest_runtime_sha:\s+`[0-9a-f]{7,40}`')
    Add-Fb2PromptValidationCheck $checks "server deploy contains latest runtime visible" ($Content -match 'deployed_contains_latest_runtime_sha:\s+`True`')
    Add-Fb2PromptValidationCheck $checks "server deploy ready visible" ($Content -match 'server_deploy_ready:\s+`True`')

    $requiredCommands = @(
        "refresh_status",
        "read_status_refresh",
        "generate_context_pack_sample_request",
        "validate_context_pack_sample_set",
        "validate_exported_context_pack_sample_set",
        "validate_context_projection_log",
        "validate_user_scenario_audit",
        "validate_current_state",
        "validate_public_contract_status",
        "validate_server_deploy_status",
        "validate_project_direct_network_policy",
        "validate_context_format_route",
        "validate_read_only_direct_read",
        "validate_gap_action_board",
        "validate_evidence_freshness",
        "validate_evidence_privacy",
        "validate_completion_matrix",
        "validate_handoff_prompt",
        "validate_visible_answer_policy",
        "validate_live_preflight_request",
        "validate_tokenless_continuation",
        "no_write_direct_read",
        "data_only_preflight",
        "data_only_preflight_via_fb2_server_token_bridge",
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
    Add-Fb2PromptValidationCheck $checks "context projection log validator present" ($Content -match 'validate_context_projection_log.+validate-fb2-context-projection-log\.ps1')
    Add-Fb2PromptValidationCheck $checks "user scenario audit validator present" ($Content -match 'validate_user_scenario_audit.+validate-fb2-user-scenario-audit\.ps1')
    Add-Fb2PromptValidationCheck $checks "public contract status validator present" ($Content -match 'validate_public_contract_status.+fb2-public-contract-status\.ps1')
    Add-Fb2PromptValidationCheck $checks "context format route validator present" ($Content -match 'validate_context_format_route.+validate-fb2-context-format-route\.ps1')
    Add-Fb2PromptValidationCheck $checks "live preflight request validator present" ($Content -match 'validate_live_preflight_request.+validate-fb2-live-preflight-request\.ps1')
    Add-Fb2PromptValidationCheck $checks "tokenless continuation validator present" ($Content -match 'validate_tokenless_continuation.+validate-fb2-tokenless-continuation\.ps1')
    Add-Fb2PromptValidationCheck $checks "evidence privacy validator present" ($Content -match 'validate_evidence_privacy.+validate-fb2-evidence-privacy\.ps1')
    Add-Fb2PromptValidationCheck $checks "data-only preflight is no visible write" ($Content -match 'data_only_preflight.+DataOnlyAcceptance.+PreflightOnly')
    Add-Fb2PromptValidationCheck $checks "token bridge preflight is no visible write" ($Content -match 'data_only_preflight_via_fb2_server_token_bridge.+run-fb2-ai-center-token-bridge\.ps1.+RunDataOnlyPreflight')
    Add-Fb2PromptValidationCheck $checks "token bridge preflight omits fb2 password argv" ($Content -notmatch 'data_only_preflight_via_fb2_server_token_bridge.+-Fb2Password')
    Add-Fb2PromptValidationCheck $checks "visible regression is explicit" ($Content -match 'visible_regression_requires_authorization.+AllowVisibleMessages')
    Add-Fb2PromptValidationCheck $checks "voice pause is explicit" ($Content -match 'ASR/TTS final evidence.*暂停|ASR/TTS.*paused|voice_final_evidence')

    $requiredSafeWithoutSecret = @(
        "public_contract_regression",
        "status_refresh_selftest",
        "context_format_route_regression",
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
- protected_live_preflight_satisfied: `True`
- answer_source_validation_ready: `True`
- voice_deferred_by_user: `True`
- next_minimum_action: `set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly`
- totals: complete `13` / deferred `1` / incomplete `0` / total `14`

## 线上主项目
- main_base: `http://43.139.149.158:8080`
- health: `OK`
- versionName: `0.3.755`
- deployed_git_sha: `1c14bde6cd12e7af87ec7feb2cb7dc412138c2c5`
- latest_runtime_sha: `12368e2ba39b6ed8071a5e43b4c4e56091a0c18c`
- deployed_contains_latest_runtime_sha: `True`
- server_deploy_ready: `True`
- note: `This verifies the deployed main-project server contains the latest runtime commit.`

## Owner 下一步
- main_project: `keep_contract_and_status_regressions_green_until_FB2_AI_CENTER_TOKEN_is_available`
- fb2_project: `provide_FB2_AI_CENTER_TOKEN_or_export_equivalent_live_Context_Pack_permission_quality_evidence`
- shared: `run_DataOnlyAcceptance_PreflightOnly_with_token_then_refresh_status_refresh_current_json`

## 计划能力 / 非生产边界
| id | status | contract | production_grounding | blocks_data_goal | answer_time_vector_candidates_enabled | next |
|---|---|---|---|---|---|---|
| p4_vector | contract_design_committed_embedding_not_started | fb2_p4_vector_contract_v1 / fb2_p4_vector_readiness_plan_v1 / fb2_p4_embedding_build_dry_run_v1 | False | False | False | dry_run_available_no_writes; writes_vector_store=false; candidate_rows_require_live_hydration=true. |

## fb2 导出样本
- attempted: `True` / complete: `True` / passed: `4` / failed: `0`
| scenario | audit | sources | business | quality_history | sha256 |
|---|---|---:|---|---|---|
| today_matches_context_pack | audit-today | 23 | match, odds, context_audit | opinion_result_review_summary | aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa |

## 可执行命令
- `refresh_status`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-refresh-current-status.ps1`
- `read_status_refresh`: `Get-Content -Raw -LiteralPath target\fb2-ai-center\status-refresh-current.json | ConvertFrom-Json`
- `generate_context_pack_sample_request`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -PrintExportRequest -ExternalUserId <fb2_user_uuid_with_orders> -OutputPath target\fb2-ai-center\context-pack-sample-request-current.json`
- `validate_context_pack_sample_set`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\context-pack-samples-validation-current.json`
- `validate_exported_context_pack_sample_set`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-pack.ps1 -ValidateSampleSet -SamplesDir <fb2_repo>\target\fb2-ai-center\samples -OutputPath target\fb2-ai-center\fb2-repo-context-pack-samples-validation-current.json`
- `validate_context_projection_log`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-projection-log.ps1 -StatusPath target\fb2-ai-center\status-current.json -OutputPath target\fb2-ai-center\context-projection-log-validation-current.json`
- `validate_user_scenario_audit`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-user-scenario-audit.ps1 -StatusPath target\fb2-ai-center\status-current.json -OutputPath target\fb2-ai-center\user-scenario-audit-validation-current.json`
- `validate_current_state`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-current-state.ps1`
- `validate_public_contract_status`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-public-contract-status.ps1 -OutputPath target\fb2-ai-center\public-contract-status-current.json`
- `validate_server_deploy_status`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-main-server-deploy-status.ps1`
- `validate_project_direct_network_policy`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-project-direct-network-policy.ps1 -OutputPath target\fb2-ai-center\project-direct-network-policy-validation-current.json`
- `validate_context_format_route`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-context-format-route.ps1 -OutputPath target\fb2-ai-center\context-format-route-validation-current.json`
- `validate_read_only_direct_read`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-readonly-summary.ps1 -SummaryPath target\fb2-ai-center\read-only-direct-read-current.json`
- `validate_gap_action_board`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-gap-action-board.ps1`
- `validate_evidence_freshness`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-evidence-freshness.ps1`
- `validate_evidence_privacy`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-evidence-privacy.ps1`
- `validate_completion_matrix`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-completion-matrix.ps1`
- `validate_handoff_prompt`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-handoff-prompt.ps1`
- `validate_visible_answer_policy`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-visible-answer-policy.ps1 -SummaryPath <DATA_ONLY_ACCEPTANCE_JSON>`
- `validate_live_preflight_request`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-live-preflight-request.ps1 -StatusPath target\fb2-ai-center\status-current.json`
- `validate_tokenless_continuation`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-tokenless-continuation.ps1 -OutputPath target\fb2-ai-center\tokenless-continuation-validation-current.json`
- `no_write_direct_read`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD>`
- `data_only_preflight`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD> -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>`
- `data_only_preflight_via_fb2_server_token_bridge`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\run-fb2-ai-center-token-bridge.ps1 -RunDataOnlyPreflight`
- `visible_regression_requires_authorization`: `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD> -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>`

## 阻塞与边界
- external_secret: `FB2_AI_CENTER_TOKEN`
- blocked_by_external_secret: `True`
- safe_to_continue_without_secret: `public_contract_regression, status_refresh_selftest, context_format_route_regression, offline_context_pack_sample_validation, handoff_documentation`
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
    $badBoundary = $good -replace 'public_contract_regression, status_refresh_selftest, context_format_route_regression, offline_context_pack_sample_validation, handoff_documentation', 'public_contract_regression, status_refresh_selftest'
    $badSourceClassification = $good -replace 'match, odds, context_audit \| opinion_result_review_summary', 'match, odds, context_audit, opinion_result_review_summary | '

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
    $badSourceClassificationResult = New-Fb2PromptValidation -Content $badSourceClassification -SourcePath "selftest-source-classification.md"
    if ([bool]$badSourceClassificationResult.success) { $failed++ }

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
