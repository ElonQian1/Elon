#requires -Version 7.0

param(
    [string]$RepoRoot = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2PromptRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2PromptPath {
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

function Read-Fb2PromptFile {
    param(
        [string]$Root,
        [string]$RelativePath
    )

    $path = Join-Path $Root $RelativePath
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Required source file not found: $path"
    }
    Get-Content -LiteralPath $path -Raw
}

function Test-Fb2PromptRegex {
    param(
        [string]$Text,
        [string]$Pattern
    )

    [regex]::IsMatch($Text, $Pattern, [System.Text.RegularExpressions.RegexOptions]::Singleline)
}

function Add-Fb2PromptCheck {
    param(
        [System.Collections.ArrayList]$Checks,
        [string]$Name,
        [bool]$Passed,
        [string]$Details = "",
        [string]$File = ""
    )

    [void]$Checks.Add([ordered]@{
        name = $Name
        passed = $Passed
        file = $File
        details = $Details
    })
}

function New-Fb2MainPromptProjectionValidation {
    param([string]$Root)

    $files = [ordered]@{
        social_ai = "server\src\social_ai.rs"
        selected_reply = "server\src\social_ai_message_reply.rs"
        context_budget = "server\src\external_app_context_budget.rs"
        tool_prompt = "server\src\external_app_context_tool_prompt.rs"
        scenario_prompt = "server\src\external_app_context_scenario_prompt.rs"
        gap_notice = "server\src\external_app_context_gap_notice.rs"
        source_validation = "server\src\external_app_context_source_validation.rs"
        answer_policy = "server\src\external_app_context_answer_policy.rs"
        tool_contract = "server\src\external_app_context_tools.rs"
    }
    $texts = [ordered]@{}
    foreach ($entry in $files.GetEnumerator()) {
        $texts[$entry.Key] = Read-Fb2PromptFile -Root $Root -RelativePath $entry.Value
    }

    $checks = [System.Collections.ArrayList]::new()

    Add-Fb2PromptCheck $checks `
        "group @EL prompt includes formatted external context" `
        ($texts.social_ai.Contains('let external_context_block = format_external_context(external_context, external_tool_results);') -and $texts.social_ai.Contains('external_context_block')) `
        "format_external_context is inserted into prompt_text" $files.social_ai
    Add-Fb2PromptCheck $checks `
        "selected-message prompt includes same external context projection" `
        ($texts.selected_reply.Contains('crate::social_ai::format_external_context(external_context, external_tool_results)') -and $texts.selected_reply.Contains('external_context_section')) `
        "selected reply reuses social_ai::format_external_context" $files.selected_reply
    Add-Fb2PromptCheck $checks `
        "social AI post-generation enforces fb2 answer shape" `
        ($texts.social_ai.Contains('ensure_fb2_grounded_answer_shape') -and $texts.social_ai.Contains('ensure_fb2_context_gap_notice') -and $texts.social_ai.Contains('ensure_fb2_opinion_memory_source')) `
        "shape + gap + opinion-memory guards" $files.social_ai

    Add-Fb2PromptCheck $checks `
        "format_external_context combines Context Pack, scenario guidance, and executed tools" `
        (Test-Fb2PromptRegex $texts.social_ai 'context_block[^;]+prompt_context_block[\s\S]+tool_block[^;]+prompt_executed_tools_block[\s\S]+scenario_block[^;]+prompt_domain_scenario_guidance[\s\S]+\[context_block,\s*scenario_block,\s*tool_block\]') `
        "context + scenario + tool blocks" $files.social_ai

    foreach ($field in @('usage_policy', 'answer_policy', 'context_quality', 'context_gap_summary', 'context_budget', 'external_metrics', 'context_fact_summary', 'context_audit_id')) {
        Add-Fb2PromptCheck $checks `
            "Context Pack prompt metadata includes $field" `
            ($texts.context_budget.Contains($field)) `
            $field $files.context_budget
    }
    Add-Fb2PromptCheck $checks `
        "Context Pack prompt preserves XML wrapper boundary" `
        ($texts.context_budget.Contains('<external_app_context source=') -and $texts.context_budget.Contains('</external_app_context>')) `
        "external_app_context XML boundary" $files.context_budget
    Add-Fb2PromptCheck $checks `
        "Context Pack prompt includes available tools and answer rules" `
        ($texts.context_budget.Contains('prompt_tool_contract_block(context)') -and $texts.context_budget.Contains('prompt_answer_rules_block(context)')) `
        "tool contract + answer rules" $files.context_budget
    Add-Fb2PromptCheck $checks `
        "Context Pack budget trims heavy fields before prompt projection" `
        ($texts.context_budget.Contains('trim_heavy_field(&mut context, "group_messages", 24)') -and $texts.context_budget.Contains('trim_heavy_field(&mut context, "user_orders", 12)') -and $texts.context_budget.Contains('trim_context_pack(&mut context, max_chars / 2)')) `
        "group/messages/orders/context_pack trimming" $files.context_budget

    foreach ($field in @('match_count', 'user_order_count', 'group_message_count', 'source_id_samples', 'citation_source_samples', 'user_order_samples', 'current_user_only_after_external_user_id_header_check')) {
        Add-Fb2PromptCheck $checks `
            "context_fact_summary includes $field" `
            ($texts.context_budget.Contains($field)) `
            $field $files.context_budget
    }
    foreach ($field in @('fact_answer_allowed', 'required_user_notice', 'business_data_available', 'truncation', 'fb2_context_gap_or_unverified_data_present')) {
        Add-Fb2PromptCheck $checks `
            "context_gap_summary includes $field" `
            ($texts.context_budget.Contains($field)) `
            $field $files.context_budget
    }

    Add-Fb2PromptCheck $checks `
        "executed tools prompt uses XML block and tool result rules" `
        ($texts.tool_prompt.Contains('<executed_external_app_tools') -and $texts.tool_prompt.Contains('<tool_result_rules>') -and $texts.tool_prompt.Contains('</executed_external_app_tools>')) `
        "executed_external_app_tools + tool_result_rules" $files.tool_prompt
    Add-Fb2PromptCheck $checks `
        "executed tools prompt exposes fact and gap summaries before full JSON" `
        ($texts.tool_prompt.Contains('<tool_fact_summary>') -and $texts.tool_prompt.Contains('<tool_gap_summary>') -and $texts.tool_prompt.Contains('MAX_PROMPT_TOOL_JSON_CHARS')) `
        "tool_fact_summary + tool_gap_summary + truncation" $files.tool_prompt
    foreach ($rule in @('grounding.status=grounded', 'grounding.status=weak', 'grounding.status=unsafe', 'current_user_only', 'match_focused_brief', 'single_group_lightweight_memory', 'single_group_persistent_opinion_index')) {
        Add-Fb2PromptCheck $checks `
            "executed tools prompt states $rule boundary" `
            ($texts.tool_prompt.Contains($rule)) `
            $rule $files.tool_prompt
    }

    Add-Fb2PromptCheck $checks `
        "domain scenario guidance is injected as XML with source rules" `
        ($texts.scenario_prompt.Contains('<fb2_domain_scenario_guidance schema=') -and $texts.scenario_prompt.Contains('fb2.domain_scenario_prompt.v1') -and $texts.scenario_prompt.Contains('<scenario_rules>') -and $texts.scenario_prompt.Contains('不能发明 match_id')) `
        "fb2_domain_scenario_guidance + scenario_rules" $files.scenario_prompt
    foreach ($scenario in @('today_matches_analysis', 'my_ticket_analysis', 'platform_order_risk', 'group_opinion_summary', 'selected_message_review', 'source_reference_audit')) {
        Add-Fb2PromptCheck $checks `
            "domain scenario guidance covers $scenario" `
            ($texts.scenario_prompt.Contains($scenario)) `
            $scenario $files.scenario_prompt
    }

    Add-Fb2PromptCheck $checks `
        "gap notice post-processor prevents missing data fabrication" `
        ($texts.gap_notice.Contains('ensure_fb2_context_gap_notice') -and $texts.gap_notice.Contains('不能把缺失数据编造成比赛、赔率、订单或群友观点事实') -and $texts.gap_notice.Contains('数据缺口：')) `
        "deterministic data gap notice" $files.gap_notice
    foreach ($reason in @('fb2_readiness_blocked', 'fb2_budget_empty', 'fb2_budget_too_large', 'missing_context_pack')) {
        Add-Fb2PromptCheck $checks `
            "gap notice reacts to $reason" `
            ($texts.gap_notice.Contains($reason)) `
            $reason $files.gap_notice
    }

    Add-Fb2PromptCheck $checks `
        "source validation summarizes missing and unmatched answer sources" `
        ($texts.source_validation.Contains('external_app.answer_source_validation.v1') -and $texts.source_validation.Contains('no_explicit_source_ids') -and $texts.source_validation.Contains('unmatched')) `
        "answer source validation statuses" $files.source_validation
    Add-Fb2PromptCheck $checks `
        "source validation only allows Context Pack, selected-message extras, audit id, or grounded/weak tools" `
        ($texts.source_validation.Contains('Context Pack source registry, selected-message extras, the context audit id, or grounded/weak tool results') -and $texts.source_validation.Contains('Some("grounded" | "weak")')) `
        "allowed source rule" $files.source_validation

    Add-Fb2PromptCheck $checks `
        "answer policy exposes prompt answer rules" `
        ($texts.answer_policy.Contains('<answer_rules>') -and $texts.answer_policy.Contains('不能编造') -and $texts.answer_policy.Contains('不保证命中')) `
        "answer_rules block" $files.answer_policy
    Add-Fb2PromptCheck $checks `
        "tool contract prompt forbids invented tool results" `
        ($texts.tool_contract.Contains('<available_external_app_tools') -and $texts.tool_contract.Contains('不能编造工具返回结果') -and $texts.tool_contract.Contains('用户订单只能查询当前用户自己的数据')) `
        "available_external_app_tools rules" $files.tool_contract

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.prompt_projection_validation.v1"
        repo_root = $Root
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
        covered_prompt_blocks = @("external_app_context", "metadata", "answer_rules", "fb2_domain_scenario_guidance", "executed_external_app_tools", "tool_fact_summary", "tool_gap_summary")
        covered_post_generation_guards = @("grounded_answer_shape", "context_gap_notice", "opinion_memory_source", "answer_source_validation")
        note = "Static guard for fb2 AI prompt projection. It proves that compact Context Pack metadata, source-aware scenario guidance, tool grounding rules, and post-generation source/gap safeguards stay wired into visible chat answers."
    }
}

function New-Fb2PromptProjectionFixture {
    param([string]$Root)

    $serverSrc = Join-Path $Root "server\src"
    New-Item -ItemType Directory -Force -Path $serverSrc | Out-Null
    Set-Content -LiteralPath (Join-Path $serverSrc "social_ai.rs") -Encoding UTF8 -Value @'
fn build_reply() {
    let external_context_block = format_external_context(external_context, external_tool_results);
    let prompt_text = format!("{} {}", external_context_block, "answer");
    ensure_fb2_grounded_answer_shape(&reply, external_context);
    crate::external_app_context_gap_notice::ensure_fb2_context_gap_notice(&reply, external_context);
    ensure_fb2_opinion_memory_source(&reply, external_context, external_tool_results);
}
pub(crate) fn format_external_context() -> String {
    let context_block = crate::external_app_context_budget::prompt_context_block(external_context);
    let tool_block = crate::external_app_context_tool_prompt::prompt_executed_tools_block(external_tool_results);
    let scenario_block = crate::external_app_context_scenario_prompt::prompt_domain_scenario_guidance(external_context, external_tool_results);
    [context_block, scenario_block, tool_block].join("\n")
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "social_ai_message_reply.rs") -Encoding UTF8 -Value @'
fn build_selected_reply() {
    let external_context_block = crate::social_ai::format_external_context(external_context, external_tool_results);
    let external_context_section = format!("{}", external_context_block);
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "external_app_context_budget.rs") -Encoding UTF8 -Value @'
fn budgeted_context() {
    trim_heavy_field(&mut context, "group_messages", 24);
    trim_heavy_field(&mut context, "user_orders", 12);
    trim_context_pack(&mut context, max_chars / 2);
}
fn prompt_context_block() {
    let tool_contract = prompt_tool_contract_block(context);
    let answer_rules = prompt_answer_rules_block(context);
    "usage_policy answer_policy context_quality context_gap_summary context_budget external_metrics context_fact_summary context_audit_id";
    "<external_app_context source=\"fb2\"></external_app_context>";
}
fn context_fact_summary() {
    "match_count user_order_count group_message_count source_id_samples citation_source_samples user_order_samples current_user_only_after_external_user_id_header_check";
}
fn context_gap_summary() {
    "fact_answer_allowed required_user_notice business_data_available truncation fb2_context_gap_or_unverified_data_present";
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "external_app_context_tool_prompt.rs") -Encoding UTF8 -Value @'
const MAX_PROMPT_TOOL_JSON_CHARS: usize = 6000;
fn prompt_executed_tools_block() {
    "<executed_external_app_tools><tool_fact_summary></tool_fact_summary><tool_gap_summary></tool_gap_summary><tool_result_rules></tool_result_rules></executed_external_app_tools>";
    "grounding.status=grounded grounding.status=weak grounding.status=unsafe current_user_only match_focused_brief single_group_lightweight_memory single_group_persistent_opinion_index";
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "external_app_context_scenario_prompt.rs") -Encoding UTF8 -Value @'
fn prompt_domain_scenario_guidance() {
    "<fb2_domain_scenario_guidance schema=\"fb2.domain_scenario_prompt.v1\"><scenario_rules>不能发明 match_id</scenario_rules></fb2_domain_scenario_guidance>";
    "today_matches_analysis my_ticket_analysis platform_order_risk group_opinion_summary selected_message_review source_reference_audit";
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "external_app_context_gap_notice.rs") -Encoding UTF8 -Value @'
fn ensure_fb2_context_gap_notice() {
    "数据缺口：不能把缺失数据编造成比赛、赔率、订单或群友观点事实";
    "fb2_readiness_blocked fb2_budget_empty fb2_budget_too_large missing_context_pack";
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "external_app_context_source_validation.rs") -Encoding UTF8 -Value @'
fn answer_source_validation_summary() {
    "external_app.answer_source_validation.v1 no_explicit_source_ids unmatched Context Pack source registry, selected-message extras, the context audit id, or grounded/weak tool results";
    Some("grounded" | "weak");
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "external_app_context_answer_policy.rs") -Encoding UTF8 -Value @'
fn prompt_answer_rules_block() {
    "<answer_rules>不能编造 不保证命中</answer_rules>";
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "external_app_context_tools.rs") -Encoding UTF8 -Value @'
fn prompt_tool_contract_block() {
    "<available_external_app_tools>不能编造工具返回结果 用户订单只能查询当前用户自己的数据</available_external_app_tools>";
}
'@
}

function Invoke-Fb2PromptProjectionSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-prompt-projection-selftest-" + [guid]::NewGuid().ToString("N"))
    try {
        New-Fb2PromptProjectionFixture -Root $tempRoot
        $failed = 0
        $good = New-Fb2MainPromptProjectionValidation -Root $tempRoot
        if (-not [bool]$good.success) {
            $good | ConvertTo-Json -Depth 8
            $failed++
        }

        $budgetPath = Join-Path $tempRoot "server\src\external_app_context_budget.rs"
        (Get-Content -LiteralPath $budgetPath -Raw).Replace('context_gap_summary', 'context_gap_missing') |
            Set-Content -LiteralPath $budgetPath -Encoding UTF8
        $bad = New-Fb2MainPromptProjectionValidation -Root $tempRoot
        if ([bool]$bad.success) {
            $failed++
        }
        if (@($bad.failed | Where-Object { [string]$_.name -eq "Context Pack prompt metadata includes context_gap_summary" }).Count -eq 0) {
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
    Invoke-Fb2PromptProjectionSelfTest
    exit 0
}

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = Get-Fb2PromptRepoRoot
} else {
    $RepoRoot = Resolve-Fb2PromptPath -Path $RepoRoot -Root (Get-Fb2PromptRepoRoot)
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $RepoRoot "target\fb2-ai-center\prompt-projection-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2PromptPath -Path $OutputPath -Root $RepoRoot
}

$result = New-Fb2MainPromptProjectionValidation -Root $RepoRoot
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
