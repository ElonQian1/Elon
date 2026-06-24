#requires -Version 7.0

param(
    [string]$RepoRoot = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2FormatRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2FormatPath {
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

function Read-Fb2FormatFile {
    param(
        [string]$Root,
        [string]$RelativePath
    )

    $path = Join-Path $Root $RelativePath
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Required file not found: $path"
    }
    Get-Content -LiteralPath $path -Raw
}

function Add-Fb2FormatCheck {
    param(
        [System.Collections.ArrayList]$Checks,
        [string]$Name,
        [bool]$Passed,
        [string]$File = "",
        [string]$Details = ""
    )

    [void]$Checks.Add([ordered]@{
        name = $Name
        passed = $Passed
        file = $File
        details = $Details
    })
}

function Test-Fb2FormatAllTokens {
    param(
        [string]$Text,
        [string[]]$Tokens
    )

    foreach ($token in $Tokens) {
        if (-not $Text.Contains($token)) {
            return $false
        }
    }
    return $true
}

function New-Fb2ContextFormatRouteValidation {
    param([string]$Root)

    $files = [ordered]@{
        repo_map_reference = "docs\repo map模块问题\repo map格式建议.md"
        symbol_index_reference = "docs\符号索引讨论\项目理解与讨论.md"
        plan = "docs\fb2-ai-center\PLAN.md"
        contracts = "docs\fb2-ai-center\contracts.md"
        readme = "docs\fb2-ai-center\README.md"
        handoff = "docs\fb2-ai-center\handoff.md"
        data_tools = "docs\fb2-ai-center\data-tools.md"
        test_plan = "docs\fb2-ai-center\test-plan.md"
        projection = "server\src\external_app_context_projection.rs"
        pack_template = "server\src\external_app_context_pack_template.rs"
        index_contract = "server\src\external_app_context_index_contract.rs"
        query_intent = "server\src\external_app_context_query_intent.rs"
        public_status = "scripts\fb2-public-contract-status.ps1"
        smoke = "scripts\smoke-fb2-ai-center.ps1"
    }
    $texts = [ordered]@{}
    foreach ($entry in $files.GetEnumerator()) {
        $texts[$entry.Key] = Read-Fb2FormatFile -Root $Root -RelativePath $entry.Value
    }

    $checks = [System.Collections.ArrayList]::new()

    Add-Fb2FormatCheck $checks `
        "repo-map reference prefers markdown plus XML/JSON boundaries" `
        (Test-Fb2FormatAllTokens $texts.repo_map_reference @("Markdown", "XML", "JSON")) `
        $files.repo_map_reference `
        "Reference guidance must keep model input clean and structured."

    Add-Fb2FormatCheck $checks `
        "symbol-index reference discourages full RAG first" `
        (Test-Fb2FormatAllTokens $texts.symbol_index_reference @("不要一开始", "repo map", "测试")) `
        $files.symbol_index_reference `
        "Reference guidance must keep first phase layered and testable."

    Add-Fb2FormatCheck $checks `
        "plan fixes first phase delivery route" `
        (Test-Fb2FormatAllTokens $texts.plan @("XML-wrapped Markdown Context Pack", "REST Context Pack", "tool manifest", "MCP/RAG")) `
        $files.plan

    Add-Fb2FormatCheck $checks `
        "contracts expose fb2_context_pack and mcp boundary" `
        (Test-Fb2FormatAllTokens $texts.contracts @("<fb2_context_pack>", "XML-wrapped Markdown", "MCP/RAG 不是当前完成条件", "citation_sources")) `
        $files.contracts

    Add-Fb2FormatCheck $checks `
        "readme records cross-project p4-lite coordination boundary" `
        (Test-Fb2FormatAllTokens $texts.readme @("fb2_context_retrieval_trace_v1", "fb2_p4_lite_candidate_retrieval_v1", "match_context_index", "answer_time_refresh_allowed=false", "maintenance_rest")) `
        $files.readme `
        "Main-project handoff must distinguish read-only evidence from fb2 maintenance refresh jobs."

    Add-Fb2FormatCheck $checks `
        "readme records p4 vector readiness as planned only" `
        (Test-Fb2FormatAllTokens $texts.readme @("9aa581e1", "fb2_p4_vector_readiness_plan_v1", "fb2_p4_vector_contract_v1", "fb2_p4_source_enumerator_v1", "fb2_p4_chunk_manifest_v1", "fb2_p4_embedding_build_dry_run_v1", "source_specific_no_write_sample_available", "id_only_no_write_manifest_available", "not_checked_missing_current_user_scope", "not_checked_missing_platform_scope", "contract_design_committed_embedding_not_started", "dry_run_available_no_writes", "production_grounding=false", "answer_time_vector_candidates_enabled=false", "writes_chunk_manifest_file=false", "persists_manifest_rows=false", "source_payload_included=false", "embedding_text_included=false", "writes_embedding_rows=false", "writes_vector_store=false", "writes_public_group_messages=false", "ready_to_write_embeddings=false", "ready_for_shadow_eval=false", "refresh_operations_used=false", "candidate_rows_require_live_hydration=true", "vector_rows_are_model_input=false", "does_not_enable_vector=true", "ready_to_build_embedding_store=false", "ready_to_enable_answer_time_vector_candidates=false", "生产 grounding")) `
        $files.readme `
        "Main-project handoff must treat fb2 P4 vector readiness and contract design as planned-only, not production grounding."

    Add-Fb2FormatCheck $checks `
        "handoff records current fb2 repository coordination evidence" `
        (Test-Fb2FormatAllTokens $texts.handoff @("1b0cdc2b", "01972c2d", "fbe2c857", "opinion_result_review_surface", "ready_quality_threshold_passed", "final gate topic", "answer-time", "refresh")) `
        $files.handoff `
        "Handoff must record the current fb2 side progress, review-quality readiness, final gate topic, and answer-time refresh boundary."

    Add-Fb2FormatCheck $checks `
        "handoff records fb2 p4 vector planned boundary" `
        (Test-Fb2FormatAllTokens $texts.handoff @("9aa581e1", "fb2_p4_vector_readiness_plan_v1", "fb2_p4_vector_contract_v1", "fb2_p4_source_enumerator_v1", "fb2_p4_chunk_manifest_v1", "fb2_p4_embedding_build_dry_run_v1", "source_specific_no_write_sample_available", "id_only_no_write_manifest_available", "not_checked_missing_current_user_scope", "not_checked_missing_platform_scope", "contract_design_committed_embedding_not_started", "dry_run_available_no_writes", "p4_vector_readiness_status=planned", "production_grounding=false", "blocks_data_goal=false", "answer_time_vector_candidates_enabled=false", "writes_chunk_manifest_file=false", "persists_manifest_rows=false", "source_payload_included=false", "embedding_text_included=false", "writes_embedding_rows=false", "writes_vector_store=false", "writes_public_group_messages=false", "ready_to_write_embeddings=false", "ready_for_shadow_eval=false", "refresh_operations_used=false", "candidate_rows_require_live_hydration=true", "vector_rows_are_model_input=false", "does_not_enable_vector=true", "ready_to_build_embedding_store=false", "ready_to_enable_answer_time_vector_candidates=false", "answer-time grounding")) `
        $files.handoff `
        "Handoff must record that fb2 P4 vector readiness/contract design is planned and not answer-time grounding."

    Add-Fb2FormatCheck $checks `
        "data-tools define domain route and retrieval evidence" `
        (Test-Fb2FormatAllTokens $texts.data_tools @("XML-wrapped Markdown Context Pack", "JSON metadata", "tool manifest/tools/execute", "fb2.retrieval_evidence_item.v1", "context_query_intent.v1")) `
        $files.data_tools

    Add-Fb2FormatCheck $checks `
        "test-plan gates route fields" `
        (Test-Fb2FormatAllTokens $texts.test_plan @("domain_data_blueprint_contract", "context_pack_template_contract", "context_query_intent_contract", "MCP 是后续包装层")) `
        $files.test_plan

    Add-Fb2FormatCheck $checks `
        "test-plan keeps p4 vector plan non-production" `
        (Test-Fb2FormatAllTokens $texts.test_plan @("fb2_p4_vector_readiness_plan_v1", "fb2_p4_vector_contract_v1", "fb2_p4_source_enumerator_v1", "fb2_p4_chunk_manifest_v1", "fb2_p4_embedding_build_dry_run_v1", "只读计划/契约设计/no-write source enumeration/ID-only no-write manifest/no-write dry-run", "不是生产 grounding", "production_grounding=false", "blocks_data_goal=false", "answer_time_vector_candidates_enabled=false", "implemented=false", "dry_run_available_no_writes", "id_only_no_write_manifest_available", "not_checked_missing_current_user_scope", "not_checked_missing_platform_scope", "writes_chunk_manifest_file=false", "persists_manifest_rows=false", "source_payload_included=false", "embedding_text_included=false", "writes_embedding_rows=false", "writes_vector_store=false", "writes_public_group_messages=false", "ready_to_write_embeddings=false", "ready_for_shadow_eval=false", "refresh_operations_used=false", "candidate_rows_require_live_hydration=true", "vector_rows_are_model_input=false", "does_not_enable_vector=true", "ready_to_build_embedding_store=false", "ready_to_enable_answer_time_vector_candidates=false")) `
        $files.test_plan `
        "P4 vector readiness/contract design must stay planned until fb2 implements embedding build, hydration, shadow eval, and answer-time readthrough."

    Add-Fb2FormatCheck $checks `
        "projection contract keeps AI-facing payload structured" `
        (Test-Fb2FormatAllTokens $texts.projection @("XML-wrapped Markdown", "xml_wrapped_markdown_context_pack_with_json_metadata", "future_wrapper_not_first_phase_fact_source", "full_database_dump", "raw_embedding_dump")) `
        $files.projection

    Add-Fb2FormatCheck $checks `
        "pack template fixes body wrapper and sections" `
        (Test-Fb2FormatAllTokens $texts.pack_template @("rest_context_pack_plus_tool_manifest_plus_tools_execute", "future_wrapper_not_first_phase_fact_source", "fb2_context_pack", "retrieval_evidence_item_shape", "minimal_markdown_template")) `
        $files.pack_template

    Add-Fb2FormatCheck $checks `
        "index contract maps internal retrieval to audited projection" `
        (Test-Fb2FormatAllTokens $texts.index_contract @("index_guides_rest_context_pack_and_tool_manifest", "future_wrapper_not_first_phase_fact_source", "project_to_xml_wrapped_markdown_context_pack", "retrieval_evidence_output_shape", "stores_fb2_business_data_in_main_project")) `
        $files.index_contract

    foreach ($indexId in @("match_index", "odds_snapshot_index", "current_user_ticket_index", "platform_order_risk_index", "group_opinion_index", "opinion_memory_index", "context_audit_index", "feedback_quality_index")) {
        Add-Fb2FormatCheck $checks "index contract includes $indexId" ($texts.index_contract.Contains($indexId)) $files.index_contract
    }

    Add-Fb2FormatCheck $checks `
        "query intent carries topic and permission shape" `
        (Test-Fb2FormatAllTokens $texts.query_intent @("fb2.context_query_intent.v1", "topic_hint", "requested_indexes", "permission_scope", "retrieval_evidence_items_reference_query_intent")) `
        $files.query_intent

    Add-Fb2FormatCheck $checks `
        "public status validates route contract" `
        (Test-Fb2FormatAllTokens $texts.public_status @("context_pack_template_contract", "domain_data_blueprint_contract", "domain_context_index_contract", "context_query_intent_contract", "retrieval_evidence_item.v1")) `
        $files.public_status

    Add-Fb2FormatCheck $checks `
        "smoke test asserts context format and route" `
        (Test-Fb2FormatAllTokens $texts.smoke @("domainDataBlueprint.context_format", "xml_wrapped_markdown_context_pack_with_json_metadata", "rest_context_pack_plus_tool_manifest_plus_tools_execute", "future_wrapper_not_first_phase_fact_source")) `
        $files.smoke

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.context_format_route_validation.v1"
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
        route_summary = [ordered]@{
            ai_facing_body = "xml_wrapped_markdown_context_pack"
            machine_metadata = "json_metadata"
            first_phase_delivery = "rest_context_pack_plus_tool_manifest_plus_tools_execute"
            mcp_status = "future_wrapper_not_first_phase_fact_source"
            no_business_data_copy_to_main_project = $true
            required_evidence_shape = "fb2.retrieval_evidence_item.v1"
            required_query_intent = "fb2.context_query_intent.v1"
            latest_fb2_coordination = "read_context_retrieval_trace_p4_lite_reports_and_opinion_result_review_quality_but_do_not_run_maintenance_refresh_during_answer_generation"
        }
        note = "Guards the fb2 AI Center context-format route derived from repo-map and symbol-index guidance: clean XML-wrapped Markdown for model-readable business projection, compact JSON metadata for machines, REST Context Pack first, MCP/RAG only as later wrappers."
    }
}

function Write-Fb2ContextFormatRouteValidation {
    param(
        [object]$Result,
        [string]$Path
    )

    $json = $Result | ConvertTo-Json -Depth 10
    if (-not [string]::IsNullOrWhiteSpace($Path)) {
        $parent = Split-Path -Parent $Path
        if (-not [string]::IsNullOrWhiteSpace($parent)) {
            New-Item -ItemType Directory -Force -Path $parent | Out-Null
        }
        Set-Content -LiteralPath $Path -Value $json -Encoding UTF8
    }
    $json
}

function Set-Fb2FormatFixtureFile {
    param(
        [string]$Root,
        [string]$RelativePath,
        [string]$Content
    )

    $path = Join-Path $Root $RelativePath
    $parent = Split-Path -Parent $path
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    Set-Content -LiteralPath $path -Value $Content -Encoding UTF8
}

function New-Fb2FormatFixtureRoot {
    $root = Join-Path ([System.IO.Path]::GetTempPath()) ("fb2-format-route-selftest-" + [guid]::NewGuid().ToString("N"))
    $common = @"
Markdown XML JSON repo map 测试 不要一开始 XML-wrapped Markdown Context Pack REST Context Pack tool manifest MCP/RAG
<fb2_context_pack> citation_sources MCP/RAG 不是当前完成条件 JSON metadata tool manifest/tools/execute fb2.retrieval_evidence_item.v1 context_query_intent.v1
domain_data_blueprint_contract context_pack_template_contract domain_context_index_contract context_query_intent_contract MCP 是后续包装层
xml_wrapped_markdown_context_pack_with_json_metadata future_wrapper_not_first_phase_fact_source full_database_dump raw_embedding_dump
rest_context_pack_plus_tool_manifest_plus_tools_execute fb2_context_pack retrieval_evidence_item_shape minimal_markdown_template
index_guides_rest_context_pack_and_tool_manifest project_to_xml_wrapped_markdown_context_pack retrieval_evidence_output_shape stores_fb2_business_data_in_main_project
fb2.context_query_intent.v1 topic_hint requested_indexes permission_scope retrieval_evidence_items_reference_query_intent
domainDataBlueprint.context_format
fb2_context_retrieval_trace_v1 fb2_p4_lite_candidate_retrieval_v1 match_context_index answer_time_refresh_allowed=false maintenance_rest 1b0cdc2b 01972c2d fbe2c857 opinion_result_review_surface ready_quality_threshold_passed final gate topic answer-time refresh
9aa581e1 d0967bb5 b2493cbc e6caccac d93287e9 165e50f2 d4efde21 fb2_p4_vector_readiness_plan_v1 fb2_p4_vector_contract_v1 fb2_p4_source_enumerator_v1 fb2_p4_chunk_manifest_v1 fb2_p4_embedding_build_dry_run_v1 source_specific_no_write_sample_available id_only_no_write_manifest_available not_checked_missing_current_user_scope not_checked_missing_platform_scope source_enumerator_checked_count source_enumerator_known_chunk_count chunk_manifest_materialized_entry_count chunk_manifest_current_scope_complete contract_design_committed_embedding_not_started dry_run_available_no_writes p4_vector_readiness_status=planned 只读计划/契约设计/no-write source enumeration/ID-only no-write manifest/no-write dry-run 只读计划报告 不是生产 grounding production_grounding=false blocks_data_goal=false answer_time_vector_candidates_enabled=false implemented=false writes_chunk_manifest_file=false persists_manifest_rows=false source_payload_included=false embedding_text_included=false writes_embedding_rows=false writes_vector_store=false writes_public_group_messages=false ready_to_write_embeddings=false ready_for_shadow_eval=false refresh_operations_used=false candidate_rows_require_live_hydration=true vector_rows_are_model_input=false does_not_enable_vector=true ready_to_build_embedding_store=false ready_to_enable_answer_time_vector_candidates=false answer-time grounding 生产 grounding source allowlist chunk schema permission filter SQL offline embedding build hydration shadow eval answer-time readthrough source_enumerations enumerator_status source_dry_runs permission_partitions known_estimated_chunk_count unknown_estimate_source_count
"@
    foreach ($relativePath in @(
            "docs\repo map模块问题\repo map格式建议.md",
            "docs\符号索引讨论\项目理解与讨论.md",
            "docs\fb2-ai-center\PLAN.md",
            "docs\fb2-ai-center\contracts.md",
            "docs\fb2-ai-center\README.md",
            "docs\fb2-ai-center\handoff.md",
            "docs\fb2-ai-center\data-tools.md",
            "docs\fb2-ai-center\test-plan.md",
            "server\src\external_app_context_projection.rs",
            "server\src\external_app_context_pack_template.rs",
            "server\src\external_app_context_index_contract.rs",
            "server\src\external_app_context_query_intent.rs",
            "scripts\fb2-public-contract-status.ps1",
            "scripts\smoke-fb2-ai-center.ps1"
        )) {
        Set-Fb2FormatFixtureFile -Root $root -RelativePath $relativePath -Content $common
    }
    $indexes = "match_index odds_snapshot_index current_user_ticket_index platform_order_risk_index group_opinion_index opinion_memory_index context_audit_index feedback_quality_index"
    Add-Content -LiteralPath (Join-Path $root "server\src\external_app_context_index_contract.rs") -Value $indexes -Encoding UTF8
    return $root
}

function Invoke-Fb2ContextFormatRouteSelfTest {
    $root = New-Fb2FormatFixtureRoot
    try {
        $failed = 0
        $good = New-Fb2ContextFormatRouteValidation -Root $root
        if (-not [bool]$good.success) {
            $good | ConvertTo-Json -Depth 8
            $failed++
        }

        Set-Content -LiteralPath (Join-Path $root "docs\fb2-ai-center\contracts.md") -Value "missing route" -Encoding UTF8
        $bad = New-Fb2ContextFormatRouteValidation -Root $root
        if ([bool]$bad.success) {
            $failed++
        }

        Write-Output "== SelfTest Summary =="
        Write-Output "failed=$failed"
        if ($failed -gt 0) {
            exit 1
        }
    } finally {
        if (Test-Path -LiteralPath $root) {
            Remove-Item -LiteralPath $root -Recurse -Force
        }
    }
}

if ($SelfTest) {
    Invoke-Fb2ContextFormatRouteSelfTest
    exit 0
}

$root = if ([string]::IsNullOrWhiteSpace($RepoRoot)) { Get-Fb2FormatRepoRoot } else { Resolve-Fb2FormatPath -Path $RepoRoot -Root (Get-Fb2FormatRepoRoot) }
$output = if ([string]::IsNullOrWhiteSpace($OutputPath)) { Join-Path $root "target\fb2-ai-center\context-format-route-validation-current.json" } else { Resolve-Fb2FormatPath -Path $OutputPath -Root $root }
$result = New-Fb2ContextFormatRouteValidation -Root $root
Write-Fb2ContextFormatRouteValidation -Result $result -Path $output | Out-Host
if (-not [bool]$result.success) {
    exit 1
}
