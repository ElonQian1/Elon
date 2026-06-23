#requires -Version 7.0

param(
    [string]$RepoRoot = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2RoutingRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2RoutingPath {
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

function Read-Fb2RoutingFile {
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

function Add-Fb2RoutingCheck {
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

function Test-Fb2RoutingRegex {
    param(
        [string]$Text,
        [string]$Pattern
    )

    return [bool]([regex]::IsMatch($Text, $Pattern, [System.Text.RegularExpressions.RegexOptions]::Singleline))
}

function Get-Fb2RoutingMatchCount {
    param(
        [string]$Text,
        [string]$Pattern
    )

    return ([regex]::Matches($Text, $Pattern, [System.Text.RegularExpressions.RegexOptions]::Singleline)).Count
}

function New-Fb2MainRuntimeRoutingValidation {
    param([string]$Root)

    $files = [ordered]@{
        social_ai = "server\src\social_ai.rs"
        selected_reply = "server\src\social_ai_message_reply.rs"
        summary_api = "server\src\group_summary_api.rs"
        external_context = "server\src\external_app_context.rs"
        tool_runtime = "server\src\external_app_context_tool_runtime.rs"
        tool_planner = "server\src\external_app_context_tool_planner.rs"
    }
    $texts = [ordered]@{}
    foreach ($entry in $files.GetEnumerator()) {
        $texts[$entry.Key] = Read-Fb2RoutingFile -Root $Root -RelativePath $entry.Value
    }

    $checks = [System.Collections.ArrayList]::new()

    Add-Fb2RoutingCheck $checks `
        "@EL group mention derives topic_hint from latest effective user text" `
        (Test-Fb2RoutingRegex $texts.social_ai 'let\s+topic_hint\s*=\s*latest_request_user_text\(&history\);') `
        "latest_request_user_text(&history)" $files.social_ai
    Add-Fb2RoutingCheck $checks `
        "@EL group mention sends topic_hint to Context Pack fetch" `
        (Test-Fb2RoutingRegex $texts.social_ai 'group_context_for_chat\([^;]+topic_hint\.as_deref\(\)') `
        "group_context_for_chat(... topic_hint.as_deref())" $files.social_ai
    Add-Fb2RoutingCheck $checks `
        "@EL group mention sends topic_hint to tool execution" `
        (Test-Fb2RoutingRegex $texts.social_ai 'group_tool_results_for_chat\([^;]+topic_hint\.as_deref\(\)') `
        "group_tool_results_for_chat(... topic_hint.as_deref())" $files.social_ai
    Add-Fb2RoutingCheck $checks `
        "@EL generated answer feedback records group mention trigger" `
        ($texts.social_ai.Contains('"group_mention"') -and $texts.social_ai.Contains('spawn_generated_answer_feedback')) `
        "trigger=group_mention" $files.social_ai

    Add-Fb2RoutingCheck $checks `
        "selected-message AI reply derives topic_hint from selected message" `
        (Test-Fb2RoutingRegex $texts.selected_reply 'let\s+topic_hint\s*=\s*selected_message_topic_hint\(&selected\);') `
        "selected_message_topic_hint(&selected)" $files.selected_reply
    Add-Fb2RoutingCheck $checks `
        "selected-message AI reply sends topic_hint to Context Pack fetch" `
        (Test-Fb2RoutingRegex $texts.selected_reply 'group_context_for_chat\([^;]+topic_hint\.as_deref\(\)') `
        "group_context_for_chat(... topic_hint.as_deref())" $files.selected_reply
    Add-Fb2RoutingCheck $checks `
        "selected-message AI reply sends topic_hint to tool execution" `
        (Test-Fb2RoutingRegex $texts.selected_reply 'group_tool_results_for_chat\([^;]+topic_hint\.as_deref\(\)') `
        "group_tool_results_for_chat(... topic_hint.as_deref())" $files.selected_reply
    Add-Fb2RoutingCheck $checks `
        "selected-message feedback uses stable selected_message_ai_reply trigger" `
        ($texts.selected_reply.Contains('"selected_message_ai_reply"') -and $texts.selected_reply.Contains('selected_message_citation_source')) `
        "trigger=selected_message_ai_reply" $files.selected_reply

    $summaryHintCount = Get-Fb2RoutingMatchCount $texts.summary_api 'summary_topic_hint\(&input\)'
    Add-Fb2RoutingCheck $checks `
        "summary-post routes manual and auto-split topics through topic_hint" `
        ($summaryHintCount -ge 2) `
        "summary_topic_hint(&input) count=$summaryHintCount" $files.summary_api
    Add-Fb2RoutingCheck $checks `
        "summary-post sends topic_hint to Context Pack fetch" `
        (Test-Fb2RoutingRegex $texts.summary_api 'group_context_for_chat\([^;]+topic_hint\.as_deref\(\)') `
        "group_context_for_chat(... topic_hint.as_deref())" $files.summary_api
    Add-Fb2RoutingCheck $checks `
        "summary-post marks generated pack entrypoint" `
        ($texts.summary_api.Contains('"group_summary_post"') -and $texts.summary_api.Contains('build_context_pack')) `
        "context_pack entrypoint=group_summary_post" $files.summary_api

    Add-Fb2RoutingCheck $checks `
        "Context Pack request targets fb2 pack endpoint" `
        ($texts.external_context.Contains('/api/main-project/context/pack')) `
        "/context/pack" $files.external_context
    Add-Fb2RoutingCheck $checks `
        "Context Pack request query includes group_id and topic_hint" `
        (Test-Fb2RoutingRegex $texts.external_context '"group_id"\.to_string\(\)[^;]+external_group_id\.to_string\(\)[^}]+topic_hint\.and_then\(clean_query_value\)[^}]+query\.push\(\("topic_hint"') `
        "group_id + topic_hint query fields" $files.external_context
    Add-Fb2RoutingCheck $checks `
        "Context Pack request includes bound external_user_id when available" `
        ($texts.external_context.Contains('"external_user_id"') -and $texts.external_context.Contains('external_account_for_main_user')) `
        "external_user_id from linked account" $files.external_context
    Add-Fb2RoutingCheck $checks `
        "Context Pack request keeps platform order scope explicit" `
        ($texts.external_context.Contains('include_platform_orders') -and $texts.external_context.Contains('platform_order_summary_requested(topic_hint)')) `
        "include_platform_orders guarded by topic/platform scope" $files.external_context
    Add-Fb2RoutingCheck $checks `
        "today-matches fallback keeps group_id and topic_hint" `
        (Test-Fb2RoutingRegex $texts.external_context '/api/main-project/context/today-matches[^}]+topic_hint\.and_then\(clean_query_value\)[^}]+query\.push\(\("topic_hint"') `
        "/today-matches fallback topic_hint" $files.external_context
    Add-Fb2RoutingCheck $checks `
        "context fetch is budgeted before prompt injection" `
        ($texts.external_context.Contains('Some(budgeted_context(context))')) `
        "budgeted_context(context)" $files.external_context
    foreach ($field in @('topic_hint_present', 'fallback_used', 'answer_policy_schema', 'context_quality_warning_count', 'tool_readiness_status')) {
        Add-Fb2RoutingCheck $checks `
            "context fetch observability logs $field" `
            ($texts.external_context.Contains($field)) `
            $field $files.external_context
    }

    Add-Fb2RoutingCheck $checks `
        "tool runtime planner receives topic_hint" `
        ($texts.tool_runtime.Contains('plan_fb2_tools(context, topic_hint)')) `
        "plan_fb2_tools(context, topic_hint)" $files.tool_runtime
    Add-Fb2RoutingCheck $checks `
        "tool runtime persists topic_hint in execution audit" `
        ($texts.tool_runtime.Contains('record_external_app_tool_execution') -and $texts.tool_runtime.Contains('topic_hint,')) `
        "ExternalAppToolExecutionWrite.topic_hint" $files.tool_runtime
    Add-Fb2RoutingCheck $checks `
        "tool runtime sends context_audit_id to fb2 tools" `
        ($texts.tool_runtime.Contains('"context_audit_id": context_audit_id')) `
        "context_audit_id in tool payload" $files.tool_runtime
    Add-Fb2RoutingCheck $checks `
        "tool runtime refuses current-user tools without linked fb2 user" `
        ($texts.tool_runtime.Contains('requires_external_user') -and $texts.tool_runtime.Contains('missing_external_user_id')) `
        "requires_external_user -> missing_external_user_id" $files.tool_runtime
    Add-Fb2RoutingCheck $checks `
        "tool runtime applies fb2 request permission headers" `
        ($texts.tool_runtime.Contains('fb2_request_context_headers(external_user_id, tool_requires_platform_scope(plan.name))')) `
        "permission headers for user/platform scope" $files.tool_runtime

    foreach ($tool in @('match_analysis_brief', 'search_user_orders', 'group_opinion_summary', 'opinion_memories', 'platform_orders', 'opinion_result_review_summary')) {
        Add-Fb2RoutingCheck $checks `
            "tool planner can route $tool" `
            ($texts.tool_planner.Contains($tool)) `
            $tool $files.tool_planner
    }
    Add-Fb2RoutingCheck $checks `
        "tool planner caps automatic tools to bounded top set" `
        ($texts.tool_planner.Contains('plans.truncate(5)')) `
        "plans.truncate(5)" $files.tool_planner
    Add-Fb2RoutingCheck $checks `
        "tool planner attaches domain scenario selection metadata" `
        ($texts.tool_planner.Contains('domain_scenario_selection') -and $texts.tool_planner.Contains('fb2_domain_scenario_selection')) `
        "domain_scenario_selection" $files.tool_planner

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.runtime_routing_validation.v1"
        repo_root = $Root
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
        covered_entrypoints = @("group_mention_at_el", "selected_message_ai_reply", "group_summary_post")
        covered_runtime_surfaces = @("context_pack_query", "today_matches_fallback", "tool_planner", "tool_execution_audit", "observability")
        note = "Static guard for the fb2 AI Center runtime route: user intent must flow from visible chat entrypoints into fb2 Context Pack requests, deterministic tool planning, audited tool execution, and privacy-safe observability."
    }
}

function New-Fb2RuntimeRoutingFixture {
    param([string]$Root)

    $serverSrc = Join-Path $Root "server\src"
    New-Item -ItemType Directory -Force -Path $serverSrc | Out-Null
    Set-Content -LiteralPath (Join-Path $serverSrc "social_ai.rs") -Encoding UTF8 -Value @'
fn reply_to_group() {
    let topic_hint = latest_request_user_text(&history);
    crate::external_app_context::group_context_for_chat(&state, &user_id, &group_id, topic_hint.as_deref());
    crate::external_app_context_tool_runtime::group_tool_results_for_chat(&state, &user_id, &group_id, context, topic_hint.as_deref());
    crate::external_app_context_feedback::spawn_generated_answer_feedback(state, user_id, group_id, id, "group_mention", external_context, external_tool_results, reply, vec![]);
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "social_ai_message_reply.rs") -Encoding UTF8 -Value @'
fn reply_to_selected_group_message() {
    let topic_hint = selected_message_topic_hint(&selected);
    crate::external_app_context::group_context_for_chat(&state, &user_id, &group_id, topic_hint.as_deref());
    crate::external_app_context_tool_runtime::group_tool_results_for_chat(&state, &user_id, &group_id, context, topic_hint.as_deref());
    crate::external_app_context_feedback::spawn_generated_answer_feedback(state, user_id, group_id, id, "selected_message_ai_reply", external_context, external_tool_results, reply, vec![selected_message_citation_source(&selected_message_id)]);
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "group_summary_api.rs") -Encoding UTF8 -Value @'
fn create_group_summary_post() {
    let topic_hint = summary_topic_hint(&input);
    crate::external_app_context::group_context_for_chat(&state, &user.id, &group_id, topic_hint.as_deref());
    build_context_pack(&group_id, &input, &messages, &documents, external_context, "group_summary_post");
}
fn auto_split_group_summary_posts() {
    let topic_hint = summary_topic_hint(&input);
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "external_app_context.rs") -Encoding UTF8 -Value @'
fn fetch_fb2_context_pack() {
    let url = "/api/main-project/context/pack";
    let mut query = vec![("group_id".to_string(), external_group_id.to_string())];
    let external_account = state.store.external_account_for_main_user(app_id, user_id);
    query.push(("external_user_id".to_string(), external_user_id.to_string()));
    if let Some(topic) = topic_hint.and_then(clean_query_value) { query.push(("topic_hint".to_string(), topic.to_string())); }
    let include_platform_orders = platform_order_summary_requested(topic_hint);
    fb2_request_context_headers(external_user_id, include_platform_orders);
    Some(budgeted_context(context))
}
fn fetch_fb2_match_context() {
    let url = "/api/main-project/context/today-matches";
    let mut query = vec![("group_id".to_string(), external_group_id.to_string())];
    if let Some(topic) = topic_hint.and_then(clean_query_value) { query.push(("topic_hint".to_string(), topic.to_string())); }
}
fn log_context_fetch() {
    let topic_hint_present = true;
    let fallback_used = true;
    let answer_policy_schema = "fb2.answer_policy.v1";
    let context_quality_warning_count = 0;
    let tool_readiness_status = "ready";
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "external_app_context_tool_runtime.rs") -Encoding UTF8 -Value @'
fn group_tool_results_for_chat() {
    let tool_plan = plan_fb2_tools(context, topic_hint);
    if plan.requires_external_user && external_user_id.is_none() { "missing_external_user_id"; }
    let payload = json!({"context_audit_id": context_audit_id});
    for (header, value) in fb2_request_context_headers(external_user_id, tool_requires_platform_scope(plan.name)) {}
    state.store.record_external_app_tool_execution(ExternalAppToolExecutionWrite { topic_hint, execution, app_id, main_group_id, external_group_id, main_user_id, external_user_id, context_audit_id });
}
'@
    Set-Content -LiteralPath (Join-Path $serverSrc "external_app_context_tool_planner.rs") -Encoding UTF8 -Value @'
fn plan() {
    "match_analysis_brief"; "search_user_orders"; "group_opinion_summary"; "opinion_memories"; "platform_orders"; "opinion_result_review_summary";
    plans.truncate(5);
    let domain_scenario_selection = fb2_domain_scenario_selection(Some(context), Some(query), &tool_names);
}
'@
}

function Invoke-Fb2RuntimeRoutingSelfTest {
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-runtime-routing-selftest-" + [guid]::NewGuid().ToString("N"))
    try {
        New-Fb2RuntimeRoutingFixture -Root $tempRoot
        $good = New-Fb2MainRuntimeRoutingValidation -Root $tempRoot
        $failed = 0
        if (-not [bool]$good.success) {
            $good | ConvertTo-Json -Depth 8
            $failed++
        }

        $selectedPath = Join-Path $tempRoot "server\src\social_ai_message_reply.rs"
        (Get-Content -LiteralPath $selectedPath -Raw).Replace('topic_hint.as_deref()', 'None') |
            Set-Content -LiteralPath $selectedPath -Encoding UTF8
        $bad = New-Fb2MainRuntimeRoutingValidation -Root $tempRoot
        if ([bool]$bad.success) {
            $failed++
        }
        if (@($bad.failed | Where-Object { [string]$_.name -eq "selected-message AI reply sends topic_hint to Context Pack fetch" }).Count -eq 0) {
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
    Invoke-Fb2RuntimeRoutingSelfTest
    exit 0
}

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = Get-Fb2RoutingRepoRoot
} else {
    $RepoRoot = Resolve-Fb2RoutingPath -Path $RepoRoot -Root (Get-Fb2RoutingRepoRoot)
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $RepoRoot "target\fb2-ai-center\runtime-routing-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2RoutingPath -Path $OutputPath -Root $RepoRoot
}

$result = New-Fb2MainRuntimeRoutingValidation -Root $RepoRoot
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
