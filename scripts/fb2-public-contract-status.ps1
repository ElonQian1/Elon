#requires -Version 7.0

param(
    [string]$MainBase = "",
    [string]$OutputPath = "",
    [int]$RequestTimeoutSec = 30,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

if (-not $MainBase) {
    $MainBase = $env:ELON_MAIN_BASE
}
if (-not $MainBase) {
    $MainBase = "http://43.139.149.158:8080"
}
$MainBase = $MainBase.TrimEnd("/")

function Set-Fb2PublicDirectNetwork {
    foreach ($name in @("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy")) {
        [System.Environment]::SetEnvironmentVariable($name, $null, "Process")
    }
    [System.Environment]::SetEnvironmentVariable("NO_PROXY", "*", "Process")
    [System.Environment]::SetEnvironmentVariable("no_proxy", "*", "Process")
}

function Invoke-Fb2PublicDirectRest {
    param(
        [string]$Uri,
        [int]$TimeoutSec
    )

    Invoke-RestMethod -Method Get -Uri $Uri -TimeoutSec $TimeoutSec -NoProxy
}

function Test-Fb2ContractContains {
    param(
        [object]$Values,
        [string]$Expected
    )

    @($Values) -contains $Expected
}

function New-Fb2PublicContractCheck {
    param(
        [string]$Id,
        [bool]$Passed,
        [string]$Detail = ""
    )

    [ordered]@{
        id = $Id
        passed = $Passed
        detail = $Detail
    }
}

function Get-Fb2PublicContractChecks {
    param(
        [object]$Health,
        [object]$Version,
        [object]$Contract
    )

    $domain = $Contract.domain_data_blueprint_contract
    $projection = $Contract.domain_context_projection_contract
    $projectionLayer = $Contract.context_projection_layer_contract
    $domainIndex = $Contract.domain_context_index_contract
    $queryIntent = $Contract.context_query_intent_contract
    $template = $Contract.context_pack_template_contract
    $toolResultEnvelope = $Contract.tool_result_envelope_contract
    $group = $Contract.group_chat_evidence_contract
    $manifest = $Contract.live_tool_manifest
    $templateSections = @($template.required_section_order)
    $templateMetadata = @($template.required_metadata)
    $templateSourceKinds = @($template.citation_source_shape.business_source_kinds)
    $templateRetrievalFields = @($template.retrieval_evidence_item_shape.required_fields)
    $projectionRetrievalFields = @($projection.retrieval_projection.item_shape.required_fields)
    $projectionLayerRetrievalFields = @($projectionLayer.retrieval_evidence_contract.required_fields)
    $domainIndexRetrievalFields = @($domainIndex.retrieval_evidence_output_shape.required_fields)
    $domainLaneIds = @($domain.lanes | ForEach-Object { $_.id })
    $domainSections = @($domain.required_context_pack_sections)
    $domainMetadata = @($domain.required_metadata)
    $domainAntiPatterns = @($domain.anti_patterns)
    $projectionLayerLaneIds = @($projectionLayer.domain_lanes | ForEach-Object { $_.id })
    $projectionLayerIndexIds = @($projectionLayer.domain_indexes | ForEach-Object { $_.id })
    $projectionLayerScenarioIds = @($projectionLayer.user_scenarios | ForEach-Object { $_.id })
    $projectionLayerForbidden = @($projectionLayer.forbidden_outputs)
    $projectionLayerNotAllowed = @($projectionLayer.ai_facing_payload.not_allowed)
    $projectionLayerGroupFields = @($projectionLayer.group_chat_evidence.required_fields)
    $domainIndexIds = @($domainIndex.indexes | ForEach-Object { $_.id })
    $domainIndexInputs = @($domainIndex.required_query_inputs)
    $domainIndexMetrics = @($domainIndex.required_metrics)
    $domainIndexNotAllowed = @($domainIndex.index_output_boundary.not_allowed)
    $queryIntentRequiredFields = @($queryIntent.request_shape.required_fields)
    $queryIntentEntrypointIds = @($queryIntent.entrypoints | ForEach-Object { $_.id })
    $queryIntentScenarioIds = @($queryIntent.scenario_intents | ForEach-Object { $_.scenario_id })
    $queryIntentAcceptanceSignals = @($queryIntent.acceptance_signals)
    $queryIntentPrivacyRules = @($queryIntent.privacy_rules)
    $groupFields = @($group.required_group_message_fields)
    $groupFlow = @($group.required_visible_flow_evidence)
    $toolIds = @($manifest.tool_ids)

    $healthOk = ([string]$Health -eq "OK") -or ([string]$Health.status -eq "ok")

    @(
        New-Fb2PublicContractCheck "main_health_ok" $healthOk ([string]$Health)
        New-Fb2PublicContractCheck "server_version_git_sha_present" (-not [string]::IsNullOrWhiteSpace([string]$Version.gitSha)) ([string]$Version.gitSha)
        New-Fb2PublicContractCheck "domain_blueprint_schema" ($domain.schema -eq "fb2.main_project.domain_data_blueprint.v1") ([string]$domain.schema)
        New-Fb2PublicContractCheck "domain_blueprint_complete" ([bool]$domain.complete) "complete=$($domain.complete)"
        New-Fb2PublicContractCheck "domain_blueprint_lane_count" ([int]$domain.lane_count -eq 6) "lane_count=$($domain.lane_count)"
        New-Fb2PublicContractCheck "domain_blueprint_no_copy" ($domain.stores_fb2_business_data_in_main_project -eq $false) "stores=$($domain.stores_fb2_business_data_in_main_project)"
        New-Fb2PublicContractCheck "domain_blueprint_rest_first" ($domain.first_phase_delivery -eq "rest_context_pack_plus_tool_manifest_plus_tools_execute") ([string]$domain.first_phase_delivery)
        New-Fb2PublicContractCheck "domain_blueprint_mcp_future" ($domain.mcp_status -eq "future_wrapper_not_first_phase_fact_source") ([string]$domain.mcp_status)
        New-Fb2PublicContractCheck "projection_layer_schema" ($projectionLayer.schema -eq "fb2.main_project.context_projection_layer.v1") ([string]$projectionLayer.schema)
        New-Fb2PublicContractCheck "projection_layer_complete" ([bool]$projectionLayer.complete) "complete=$($projectionLayer.complete)"
        New-Fb2PublicContractCheck "projection_layer_no_copy" ($projectionLayer.stores_fb2_business_data_in_main_project -eq $false) "stores=$($projectionLayer.stores_fb2_business_data_in_main_project)"
        New-Fb2PublicContractCheck "projection_layer_rest_first" ($projectionLayer.first_phase_delivery -eq "rest_context_pack_plus_tool_manifest_plus_tools_execute") ([string]$projectionLayer.first_phase_delivery)
        New-Fb2PublicContractCheck "projection_layer_mcp_future" ($projectionLayer.mcp_status -eq "future_wrapper_not_first_phase_fact_source") ([string]$projectionLayer.mcp_status)
        New-Fb2PublicContractCheck "projection_layer_wrapper" ($projectionLayer.ai_facing_payload.wrapper -eq "fb2_context_pack") ([string]$projectionLayer.ai_facing_payload.wrapper)
        New-Fb2PublicContractCheck "projection_layer_domain_lane_count" ([int]$projectionLayer.domain_lane_count -eq 6) "lane_count=$($projectionLayer.domain_lane_count)"
        New-Fb2PublicContractCheck "projection_layer_domain_index_count" ([int]$projectionLayer.domain_index_count -eq 8) "index_count=$($projectionLayer.domain_index_count)"
        New-Fb2PublicContractCheck "projection_layer_user_scenario_count" ([int]$projectionLayer.user_scenario_count -eq 7) "scenario_count=$($projectionLayer.user_scenario_count)"
        foreach ($laneId in @("match_facts_and_odds", "current_user_tickets", "platform_order_summary", "group_opinions", "opinion_learning_loop", "quality_feedback_audit")) {
            New-Fb2PublicContractCheck "projection_layer_lane_$laneId" (Test-Fb2ContractContains $projectionLayerLaneIds $laneId) ($projectionLayerLaneIds -join ",")
        }
        foreach ($indexId in @("match_index", "odds_snapshot_index", "current_user_ticket_index", "platform_order_risk_index", "group_opinion_index", "opinion_memory_index", "context_audit_index", "feedback_quality_index")) {
            New-Fb2PublicContractCheck "projection_layer_index_$indexId" (Test-Fb2ContractContains $projectionLayerIndexIds $indexId) ($projectionLayerIndexIds -join ",")
        }
        foreach ($scenarioId in @("today_matches_analysis", "my_ticket_analysis", "platform_order_risk", "group_opinion_summary", "selected_message_review", "group_discussion_summary_post", "source_reference_audit")) {
            New-Fb2PublicContractCheck "projection_layer_scenario_$scenarioId" (Test-Fb2ContractContains $projectionLayerScenarioIds $scenarioId) ($projectionLayerScenarioIds -join ",")
        }
        New-Fb2PublicContractCheck "projection_layer_forbidden_fabricated_odds" (Test-Fb2ContractContains $projectionLayerForbidden "fabricated_odds") ($projectionLayerForbidden -join ",")
        New-Fb2PublicContractCheck "projection_layer_forbidden_raw_embedding_dump" (Test-Fb2ContractContains $projectionLayerForbidden "raw_embedding_dump") ($projectionLayerForbidden -join ",")
        New-Fb2PublicContractCheck "projection_layer_not_allowed_full_database_dump" (Test-Fb2ContractContains $projectionLayerNotAllowed "full_database_dump") ($projectionLayerNotAllowed -join ",")
        New-Fb2PublicContractCheck "projection_layer_group_direct_read" ($projectionLayer.group_chat_evidence.method -eq "direct_api_read") ([string]$projectionLayer.group_chat_evidence.method)
        New-Fb2PublicContractCheck "projection_layer_group_rejects_screenshots" ($projectionLayer.group_chat_evidence.screenshots_accepted -eq $false) "screenshots_accepted=$($projectionLayer.group_chat_evidence.screenshots_accepted)"
        New-Fb2PublicContractCheck "projection_layer_group_text_sha256" (Test-Fb2ContractContains $projectionLayerGroupFields "text_sha256") ($projectionLayerGroupFields -join ",")
        New-Fb2PublicContractCheck "domain_index_schema" ($domainIndex.schema -eq "fb2.main_project.domain_context_index.v1") ([string]$domainIndex.schema)
        New-Fb2PublicContractCheck "domain_index_complete" ([bool]$domainIndex.complete) "complete=$($domainIndex.complete)"
        New-Fb2PublicContractCheck "domain_index_count" ([int]$domainIndex.index_count -eq 8) "index_count=$($domainIndex.index_count)"
        New-Fb2PublicContractCheck "domain_index_no_copy" ($domainIndex.stores_fb2_business_data_in_main_project -eq $false) "stores=$($domainIndex.stores_fb2_business_data_in_main_project)"
        New-Fb2PublicContractCheck "domain_index_match" (Test-Fb2ContractContains $domainIndexIds "match_index") ($domainIndexIds -join ",")
        New-Fb2PublicContractCheck "domain_index_odds_snapshot" (Test-Fb2ContractContains $domainIndexIds "odds_snapshot_index") ($domainIndexIds -join ",")
        New-Fb2PublicContractCheck "domain_index_user_ticket" (Test-Fb2ContractContains $domainIndexIds "current_user_ticket_index") ($domainIndexIds -join ",")
        New-Fb2PublicContractCheck "domain_index_platform_risk" (Test-Fb2ContractContains $domainIndexIds "platform_order_risk_index") ($domainIndexIds -join ",")
        New-Fb2PublicContractCheck "domain_index_group_opinion" (Test-Fb2ContractContains $domainIndexIds "group_opinion_index") ($domainIndexIds -join ",")
        New-Fb2PublicContractCheck "domain_index_opinion_memory" (Test-Fb2ContractContains $domainIndexIds "opinion_memory_index") ($domainIndexIds -join ",")
        New-Fb2PublicContractCheck "domain_index_context_audit" (Test-Fb2ContractContains $domainIndexIds "context_audit_index") ($domainIndexIds -join ",")
        New-Fb2PublicContractCheck "domain_index_feedback_quality" (Test-Fb2ContractContains $domainIndexIds "feedback_quality_index") ($domainIndexIds -join ",")
        New-Fb2PublicContractCheck "domain_index_topic_hint" (Test-Fb2ContractContains $domainIndexInputs "topic_hint") ($domainIndexInputs -join ",")
        New-Fb2PublicContractCheck "domain_index_metrics_budget" (Test-Fb2ContractContains $domainIndexMetrics "budget_status") ($domainIndexMetrics -join ",")
        New-Fb2PublicContractCheck "domain_index_no_raw_embedding_dump" (Test-Fb2ContractContains $domainIndexNotAllowed "raw_embedding_dump") ($domainIndexNotAllowed -join ",")
        New-Fb2PublicContractCheck "context_pack_template_schema" ($template.schema -eq "fb2.context_pack_template.v1") ([string]$template.schema)
        New-Fb2PublicContractCheck "context_pack_template_complete" ([bool]$template.complete) "complete=$($template.complete)"
        New-Fb2PublicContractCheck "context_pack_template_wrapper" ($template.body.wrapper -eq "fb2_context_pack") ([string]$template.body.wrapper)
        New-Fb2PublicContractCheck "context_pack_template_rest_first" ($template.first_phase_delivery -eq "rest_context_pack_plus_tool_manifest_plus_tools_execute") ([string]$template.first_phase_delivery)
        New-Fb2PublicContractCheck "context_pack_template_mcp_future" ($template.mcp_status -eq "future_wrapper_not_first_phase_fact_source") ([string]$template.mcp_status)
        New-Fb2PublicContractCheck "context_pack_template_user_order_section" (Test-Fb2ContractContains $templateSections "user_order_slice") ($templateSections -join ",")
        New-Fb2PublicContractCheck "context_pack_template_group_opinion_section" (Test-Fb2ContractContains $templateSections "group_opinion_slice") ($templateSections -join ",")
        New-Fb2PublicContractCheck "context_pack_template_retrieval_evidence_section" (Test-Fb2ContractContains $templateSections "retrieval_evidence") ($templateSections -join ",")
        New-Fb2PublicContractCheck "context_pack_template_citation_metadata" (Test-Fb2ContractContains $templateMetadata "citation_sources") ($templateMetadata -join ",")
        New-Fb2PublicContractCheck "context_pack_template_preflight_metadata" (Test-Fb2ContractContains $templateMetadata "preflight_readiness") ($templateMetadata -join ",")
        New-Fb2PublicContractCheck "context_pack_template_order_source_kind" (Test-Fb2ContractContains $templateSourceKinds "user_order") ($templateSourceKinds -join ",")
        New-Fb2PublicContractCheck "context_pack_template_opinion_source_kind" (Test-Fb2ContractContains $templateSourceKinds "opinion_memory") ($templateSourceKinds -join ",")
        New-Fb2PublicContractCheck "context_pack_template_business_sources_exclude_feedback" (-not (Test-Fb2ContractContains $templateSourceKinds "feedback")) ($templateSourceKinds -join ",")
        New-Fb2PublicContractCheck "context_pack_template_retrieval_evidence_shape_schema" ($template.retrieval_evidence_item_shape.schema -eq "fb2.retrieval_evidence_item.v1") ([string]$template.retrieval_evidence_item_shape.schema)
        foreach ($field in @("source_id", "source_kind", "lane_id", "index_id", "reason", "freshness", "permission_scope", "citation_source_id")) {
            New-Fb2PublicContractCheck "context_pack_template_retrieval_evidence_field_$field" (Test-Fb2ContractContains $templateRetrievalFields $field) ($templateRetrievalFields -join ",")
        }
        New-Fb2PublicContractCheck "domain_projection_retrieval_evidence_shape_schema" ($projection.retrieval_projection.item_shape.schema -eq "fb2.retrieval_evidence_item.v1") ([string]$projection.retrieval_projection.item_shape.schema)
        foreach ($field in @("source_id", "source_kind", "lane_id", "index_id", "reason", "freshness", "permission_scope", "citation_source_id")) {
            New-Fb2PublicContractCheck "domain_projection_retrieval_evidence_field_$field" (Test-Fb2ContractContains $projectionRetrievalFields $field) ($projectionRetrievalFields -join ",")
        }
        New-Fb2PublicContractCheck "projection_layer_retrieval_evidence_shape_schema" ($projectionLayer.retrieval_evidence_contract.schema -eq "fb2.retrieval_evidence_item.v1") ([string]$projectionLayer.retrieval_evidence_contract.schema)
        New-Fb2PublicContractCheck "projection_layer_retrieval_evidence_citation_source_id" (Test-Fb2ContractContains $projectionLayerRetrievalFields "citation_source_id") ($projectionLayerRetrievalFields -join ",")
        New-Fb2PublicContractCheck "domain_index_retrieval_evidence_shape_schema" ($domainIndex.retrieval_evidence_output_shape.schema -eq "fb2.retrieval_evidence_item.v1") ([string]$domainIndex.retrieval_evidence_output_shape.schema)
        New-Fb2PublicContractCheck "domain_index_retrieval_evidence_index_id" (Test-Fb2ContractContains $domainIndexRetrievalFields "index_id") ($domainIndexRetrievalFields -join ",")
        New-Fb2PublicContractCheck "domain_index_retrieval_evidence_citation_source_id" (Test-Fb2ContractContains $domainIndexRetrievalFields "citation_source_id") ($domainIndexRetrievalFields -join ",")
        New-Fb2PublicContractCheck "context_query_intent_schema" ($queryIntent.schema -eq "fb2.context_query_intent.v1") ([string]$queryIntent.schema)
        New-Fb2PublicContractCheck "context_query_intent_complete" ([bool]$queryIntent.complete) "complete=$($queryIntent.complete)"
        New-Fb2PublicContractCheck "context_query_intent_no_copy" ($queryIntent.stores_fb2_business_data_in_main_project -eq $false) "stores=$($queryIntent.stores_fb2_business_data_in_main_project)"
        New-Fb2PublicContractCheck "context_query_intent_scenario_count" ([int]$queryIntent.scenario_count -eq 7) "scenario_count=$($queryIntent.scenario_count)"
        foreach ($field in @("query_intent_id", "entrypoint", "scenario_id", "group_id", "topic_hint", "intent_lanes", "requested_indexes", "permission_scope", "source_request", "output_limits")) {
            New-Fb2PublicContractCheck "context_query_intent_field_$field" (Test-Fb2ContractContains $queryIntentRequiredFields $field) ($queryIntentRequiredFields -join ",")
        }
        foreach ($entrypointId in @("group_mention_at_el", "selected_message_ai_reply", "group_summary_post", "chat_bootstrap_ai_reply")) {
            New-Fb2PublicContractCheck "context_query_intent_entrypoint_$entrypointId" (Test-Fb2ContractContains $queryIntentEntrypointIds $entrypointId) ($queryIntentEntrypointIds -join ",")
        }
        foreach ($scenarioId in @("today_matches_analysis", "my_ticket_analysis", "platform_order_risk", "group_opinion_summary", "selected_message_review", "group_discussion_summary_post", "source_reference_audit")) {
            New-Fb2PublicContractCheck "context_query_intent_scenario_$scenarioId" (Test-Fb2ContractContains $queryIntentScenarioIds $scenarioId) ($queryIntentScenarioIds -join ",")
        }
        New-Fb2PublicContractCheck "context_query_intent_retrieval_evidence_link" (Test-Fb2ContractContains $queryIntentAcceptanceSignals "retrieval_evidence_items_reference_query_intent") ($queryIntentAcceptanceSignals -join ",")
        New-Fb2PublicContractCheck "context_query_intent_privacy_hash_only" (($queryIntentPrivacyRules -join "`n") -match "text_sha256" -and ($queryIntentPrivacyRules -join "`n") -match "raw group message bodies") ($queryIntentPrivacyRules -join " | ")
        New-Fb2PublicContractCheck "tool_result_answer_source_validation_schema" ($toolResultEnvelope.answer_source_validation.schema -eq "external_app.answer_source_validation.v1") ([string]$toolResultEnvelope.answer_source_validation.schema)
        New-Fb2PublicContractCheck "tool_result_answer_source_validation_tool_sources" ([string]$toolResultEnvelope.answer_source_validation.rule -match "matched_tool_source_ids") ([string]$toolResultEnvelope.answer_source_validation.rule)
        New-Fb2PublicContractCheck "tool_result_answer_source_validation_missing_sources" ([string]$toolResultEnvelope.answer_source_validation.rule -match "has_missing_explicit_sources" -and [string]$toolResultEnvelope.answer_source_validation.rule -match "no_explicit_source_ids") ([string]$toolResultEnvelope.answer_source_validation.rule)
        New-Fb2PublicContractCheck "domain_lane_current_user_tickets" (Test-Fb2ContractContains $domainLaneIds "current_user_tickets") ($domainLaneIds -join ",")
        New-Fb2PublicContractCheck "domain_lane_group_opinions" (Test-Fb2ContractContains $domainLaneIds "group_opinions") ($domainLaneIds -join ",")
        New-Fb2PublicContractCheck "domain_lane_quality_feedback_audit" (Test-Fb2ContractContains $domainLaneIds "quality_feedback_audit") ($domainLaneIds -join ",")
        New-Fb2PublicContractCheck "domain_section_group_opinion_slice" (Test-Fb2ContractContains $domainSections "group_opinion_slice") ($domainSections -join ",")
        New-Fb2PublicContractCheck "domain_metadata_citation_sources" (Test-Fb2ContractContains $domainMetadata "citation_sources") ($domainMetadata -join ",")
        New-Fb2PublicContractCheck "domain_antipattern_full_database_dump" (Test-Fb2ContractContains $domainAntiPatterns "full_database_dump") ($domainAntiPatterns -join ",")
        New-Fb2PublicContractCheck "group_chat_evidence_schema" ($group.schema -eq "fb2.main_project.group_chat_evidence.v1") ([string]$group.schema)
        New-Fb2PublicContractCheck "group_chat_direct_api_read" ($group.group_chat_test_method -eq "direct_api_read") ([string]$group.group_chat_test_method)
        New-Fb2PublicContractCheck "group_chat_rejects_screenshots" ($group.screenshots_accepted -eq $false) "screenshots_accepted=$($group.screenshots_accepted)"
        New-Fb2PublicContractCheck "group_chat_field_message_id" (Test-Fb2ContractContains $groupFields "message_id") ($groupFields -join ",")
        New-Fb2PublicContractCheck "group_chat_field_text_len" (Test-Fb2ContractContains $groupFields "text_len") ($groupFields -join ",")
        New-Fb2PublicContractCheck "group_chat_field_text_sha256" (Test-Fb2ContractContains $groupFields "text_sha256") ($groupFields -join ",")
        New-Fb2PublicContractCheck "group_chat_flow_mention_reply" (Test-Fb2ContractContains $groupFlow "visible_mention_ai_reply_read") ($groupFlow -join ",")
        New-Fb2PublicContractCheck "group_chat_flow_selected_reply" (Test-Fb2ContractContains $groupFlow "selected_message_ai_reply_read") ($groupFlow -join ",")
        New-Fb2PublicContractCheck "group_chat_flow_summary_post" (Test-Fb2ContractContains $groupFlow "summary_post_read") ($groupFlow -join ",")
        New-Fb2PublicContractCheck "group_chat_flow_feedback_quality" (Test-Fb2ContractContains $groupFlow "feedback_quality_read") ($groupFlow -join ",")
        New-Fb2PublicContractCheck "live_manifest_ready" ($manifest.status -eq "ready") ([string]$manifest.status)
        New-Fb2PublicContractCheck "live_manifest_context_pack" (Test-Fb2ContractContains $toolIds "context_pack") ($toolIds -join ",")
        New-Fb2PublicContractCheck "live_manifest_match_analysis_brief" (Test-Fb2ContractContains $toolIds "match_analysis_brief") ($toolIds -join ",")
        New-Fb2PublicContractCheck "live_manifest_group_opinion_summary" (Test-Fb2ContractContains $toolIds "group_opinion_summary") ($toolIds -join ",")
    )
}

function New-Fb2PublicContractStatus {
    param(
        [string]$Base,
        [object]$Health,
        [object]$Version,
        [object]$Contract
    )

    $checks = @(Get-Fb2PublicContractChecks -Health $Health -Version $Version -Contract $Contract)
    $failed = @($checks | Where-Object { -not $_.passed })

    [ordered]@{
        schema = "fb2.main_project.public_contract_status.v1"
        generated_at = (Get-Date).ToUniversalTime().ToString("o")
        main_base = $Base
        server = [ordered]@{
            health = if ($Health -is [string]) { $Health } else { $Health.status }
            versionName = [string]$Version.versionName
            gitSha = [string]$Version.gitSha
        }
        success = (@($failed).Count -eq 0)
        passed_count = @($checks | Where-Object { $_.passed }).Count
        failed_count = @($failed).Count
        failed_checks = @($failed | ForEach-Object { $_.id })
        checks = @($checks)
        contract_summary = [ordered]@{
            context_projection_layer_schema = [string]$Contract.context_projection_layer_contract.schema
            context_projection_layer_complete = [bool]$Contract.context_projection_layer_contract.complete
            context_projection_layer_lane_count = [int]$Contract.context_projection_layer_contract.domain_lane_count
            context_projection_layer_lane_ids = @($Contract.context_projection_layer_contract.domain_lanes | ForEach-Object { $_.id })
            context_projection_layer_index_count = [int]$Contract.context_projection_layer_contract.domain_index_count
            context_projection_layer_index_ids = @($Contract.context_projection_layer_contract.domain_indexes | ForEach-Object { $_.id })
            context_projection_layer_scenario_count = [int]$Contract.context_projection_layer_contract.user_scenario_count
            context_projection_layer_scenario_ids = @($Contract.context_projection_layer_contract.user_scenarios | ForEach-Object { $_.id })
            context_projection_layer_group_method = [string]$Contract.context_projection_layer_contract.group_chat_evidence.method
            context_projection_layer_screenshots_accepted = [bool]$Contract.context_projection_layer_contract.group_chat_evidence.screenshots_accepted
            context_projection_layer_group_fields = @($Contract.context_projection_layer_contract.group_chat_evidence.required_fields)
            context_projection_layer_retrieval_evidence_schema = [string]$Contract.context_projection_layer_contract.retrieval_evidence_contract.schema
            context_projection_layer_retrieval_evidence_fields = @($Contract.context_projection_layer_contract.retrieval_evidence_contract.required_fields)
            domain_data_blueprint_schema = [string]$Contract.domain_data_blueprint_contract.schema
            domain_projection_retrieval_evidence_schema = [string]$Contract.domain_context_projection_contract.retrieval_projection.item_shape.schema
            domain_projection_retrieval_evidence_fields = @($Contract.domain_context_projection_contract.retrieval_projection.item_shape.required_fields)
            domain_context_index_schema = [string]$Contract.domain_context_index_contract.schema
            domain_context_index_count = [int]$Contract.domain_context_index_contract.index_count
            domain_context_index_ids = @($Contract.domain_context_index_contract.indexes | ForEach-Object { $_.id })
            domain_context_index_retrieval_evidence_schema = [string]$Contract.domain_context_index_contract.retrieval_evidence_output_shape.schema
            domain_context_index_retrieval_evidence_fields = @($Contract.domain_context_index_contract.retrieval_evidence_output_shape.required_fields)
            context_query_intent_schema = [string]$Contract.context_query_intent_contract.schema
            context_query_intent_complete = [bool]$Contract.context_query_intent_contract.complete
            context_query_intent_scenario_count = [int]$Contract.context_query_intent_contract.scenario_count
            context_query_intent_scenario_ids = @($Contract.context_query_intent_contract.scenario_intents | ForEach-Object { $_.scenario_id })
            context_query_intent_entrypoint_ids = @($Contract.context_query_intent_contract.entrypoints | ForEach-Object { $_.id })
            context_query_intent_required_fields = @($Contract.context_query_intent_contract.request_shape.required_fields)
            context_pack_template_schema = [string]$Contract.context_pack_template_contract.schema
            context_pack_template_wrapper = [string]$Contract.context_pack_template_contract.body.wrapper
            context_pack_template_sections = @($Contract.context_pack_template_contract.required_section_order)
            context_pack_template_retrieval_evidence_schema = [string]$Contract.context_pack_template_contract.retrieval_evidence_item_shape.schema
            context_pack_template_retrieval_evidence_fields = @($Contract.context_pack_template_contract.retrieval_evidence_item_shape.required_fields)
            domain_lane_count = [int]$Contract.domain_data_blueprint_contract.lane_count
            stores_fb2_business_data_in_main_project = [bool]$Contract.domain_data_blueprint_contract.stores_fb2_business_data_in_main_project
            group_chat_evidence_schema = [string]$Contract.group_chat_evidence_contract.schema
            group_chat_test_method = [string]$Contract.group_chat_evidence_contract.group_chat_test_method
            screenshots_accepted = [bool]$Contract.group_chat_evidence_contract.screenshots_accepted
            required_group_message_fields = @($Contract.group_chat_evidence_contract.required_group_message_fields)
            live_tool_count = [int]$Contract.live_tool_manifest.tool_count
            answer_source_validation_schema = [string]$Contract.tool_result_envelope_contract.answer_source_validation.schema
            answer_source_validation_rule = [string]$Contract.tool_result_envelope_contract.answer_source_validation.rule
        }
        limitations = @(
            "public_contract_only_no_fb2_service_token_required",
            "does_not_verify_fb2_live_context_pack_or_orders",
            "does_not_write_or_read_visible_group_flow_beyond_public_contract",
            "does_not_replace_DataOnlyAcceptance_or_FinalAcceptance"
        )
        next_actions = @(
            "use_this_for_code_and_public_contract_status",
            "run_smoke_fb2_visible_chat_ReadOnlyDirectRead_for_group_api_evidence",
            "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly_for_live_data_permission_quality"
        )
    }
}

function Invoke-Fb2PublicContractSelfTest {
    $retrievalEvidenceFields = @(
        "evidence_id",
        "source_id",
        "source_kind",
        "section_id",
        "lane_id",
        "index_id",
        "reason",
        "freshness",
        "permission_scope",
        "citation_source_id"
    )
    $syntheticContract = [pscustomobject]@{
        domain_data_blueprint_contract = [pscustomobject]@{
            schema = "fb2.main_project.domain_data_blueprint.v1"
            complete = $true
            lane_count = 6
            stores_fb2_business_data_in_main_project = $false
            first_phase_delivery = "rest_context_pack_plus_tool_manifest_plus_tools_execute"
            mcp_status = "future_wrapper_not_first_phase_fact_source"
            lanes = @(
                [pscustomobject]@{ id = "current_user_tickets" },
                [pscustomobject]@{ id = "group_opinions" },
                [pscustomobject]@{ id = "quality_feedback_audit" }
            )
            required_context_pack_sections = @("group_opinion_slice")
            required_metadata = @("citation_sources")
            anti_patterns = @("full_database_dump")
        }
        context_projection_layer_contract = [pscustomobject]@{
            schema = "fb2.main_project.context_projection_layer.v1"
            complete = $true
            stores_fb2_business_data_in_main_project = $false
            first_phase_delivery = "rest_context_pack_plus_tool_manifest_plus_tools_execute"
            mcp_status = "future_wrapper_not_first_phase_fact_source"
            domain_lane_count = 6
            domain_lanes = @(
                [pscustomobject]@{ id = "match_facts_and_odds" },
                [pscustomobject]@{ id = "current_user_tickets" },
                [pscustomobject]@{ id = "platform_order_summary" },
                [pscustomobject]@{ id = "group_opinions" },
                [pscustomobject]@{ id = "opinion_learning_loop" },
                [pscustomobject]@{ id = "quality_feedback_audit" }
            )
            domain_index_count = 8
            domain_indexes = @(
                [pscustomobject]@{ id = "match_index" },
                [pscustomobject]@{ id = "odds_snapshot_index" },
                [pscustomobject]@{ id = "current_user_ticket_index" },
                [pscustomobject]@{ id = "platform_order_risk_index" },
                [pscustomobject]@{ id = "group_opinion_index" },
                [pscustomobject]@{ id = "opinion_memory_index" },
                [pscustomobject]@{ id = "context_audit_index" },
                [pscustomobject]@{ id = "feedback_quality_index" }
            )
            user_scenario_count = 7
            user_scenarios = @(
                [pscustomobject]@{ id = "today_matches_analysis" },
                [pscustomobject]@{ id = "my_ticket_analysis" },
                [pscustomobject]@{ id = "platform_order_risk" },
                [pscustomobject]@{ id = "group_opinion_summary" },
                [pscustomobject]@{ id = "selected_message_review" },
                [pscustomobject]@{ id = "group_discussion_summary_post" },
                [pscustomobject]@{ id = "source_reference_audit" }
            )
            forbidden_outputs = @("fabricated_odds", "raw_embedding_dump", "full_database_dump")
            ai_facing_payload = [pscustomobject]@{
                wrapper = "fb2_context_pack"
                not_allowed = @("raw_html_prompt", "full_database_dump", "raw_embedding_dump")
            }
            group_chat_evidence = [pscustomobject]@{
                method = "direct_api_read"
                screenshots_accepted = $false
                required_fields = @("message_id", "type", "sender_id", "created_at", "text_len", "text_sha256")
            }
            retrieval_evidence_contract = [pscustomobject]@{
                schema = "fb2.retrieval_evidence_item.v1"
                required_fields = $retrievalEvidenceFields
            }
        }
        domain_context_projection_contract = [pscustomobject]@{
            retrieval_projection = [pscustomobject]@{
                item_shape = [pscustomobject]@{
                    schema = "fb2.retrieval_evidence_item.v1"
                    required_fields = $retrievalEvidenceFields
                }
            }
        }
        domain_context_index_contract = [pscustomobject]@{
            schema = "fb2.main_project.domain_context_index.v1"
            complete = $true
            index_count = 8
            stores_fb2_business_data_in_main_project = $false
            indexes = @(
                [pscustomobject]@{ id = "match_index" },
                [pscustomobject]@{ id = "odds_snapshot_index" },
                [pscustomobject]@{ id = "current_user_ticket_index" },
                [pscustomobject]@{ id = "platform_order_risk_index" },
                [pscustomobject]@{ id = "group_opinion_index" },
                [pscustomobject]@{ id = "opinion_memory_index" },
                [pscustomobject]@{ id = "context_audit_index" },
                [pscustomobject]@{ id = "feedback_quality_index" }
            )
            required_query_inputs = @("group_id", "topic_hint", "external_user_id_when_user_orders_are_requested")
            required_metrics = @("index_latency_ms", "budget_status")
            index_output_boundary = [pscustomobject]@{
                not_allowed = @("raw_embedding_dump", "full_database_dump")
            }
            retrieval_evidence_output_shape = [pscustomobject]@{
                schema = "fb2.retrieval_evidence_item.v1"
                required_fields = $retrievalEvidenceFields
            }
        }
        context_pack_template_contract = [pscustomobject]@{
            schema = "fb2.context_pack_template.v1"
            complete = $true
            first_phase_delivery = "rest_context_pack_plus_tool_manifest_plus_tools_execute"
            mcp_status = "future_wrapper_not_first_phase_fact_source"
            body = [pscustomobject]@{
                wrapper = "fb2_context_pack"
            }
            required_section_order = @(
                "usage_boundary",
                "match_facts",
                "user_order_slice",
                "platform_order_summary",
                "group_opinion_slice",
                "retrieval_evidence",
                "quality_feedback"
            )
            required_metadata = @("context_pack_version", "generated_at", "context_audit_id", "citation_sources", "preflight_readiness")
            citation_source_shape = [pscustomobject]@{
                business_source_kinds = @("context_audit", "match", "odds", "user_order", "ticket", "group_message", "opinion_memory", "platform_order_summary")
            }
            retrieval_evidence_item_shape = [pscustomobject]@{
                schema = "fb2.retrieval_evidence_item.v1"
                required_fields = $retrievalEvidenceFields
            }
        }
        context_query_intent_contract = [pscustomobject]@{
            schema = "fb2.context_query_intent.v1"
            complete = $true
            stores_fb2_business_data_in_main_project = $false
            scenario_count = 7
            request_shape = [pscustomobject]@{
                required_fields = @(
                    "query_intent_id",
                    "entrypoint",
                    "scenario_id",
                    "group_id",
                    "topic_hint",
                    "intent_lanes",
                    "requested_indexes",
                    "permission_scope",
                    "source_request",
                    "output_limits"
                )
            }
            entrypoints = @(
                [pscustomobject]@{ id = "group_mention_at_el" },
                [pscustomobject]@{ id = "selected_message_ai_reply" },
                [pscustomobject]@{ id = "group_summary_post" },
                [pscustomobject]@{ id = "chat_bootstrap_ai_reply" }
            )
            scenario_intents = @(
                [pscustomobject]@{ scenario_id = "today_matches_analysis" },
                [pscustomobject]@{ scenario_id = "my_ticket_analysis" },
                [pscustomobject]@{ scenario_id = "platform_order_risk" },
                [pscustomobject]@{ scenario_id = "group_opinion_summary" },
                [pscustomobject]@{ scenario_id = "selected_message_review" },
                [pscustomobject]@{ scenario_id = "group_discussion_summary_post" },
                [pscustomobject]@{ scenario_id = "source_reference_audit" }
            )
            acceptance_signals = @("retrieval_evidence_items_reference_query_intent")
            privacy_rules = @(
                "Do not copy fb2 raw databases, embeddings, full order rows, raw group message bodies, real tokens, or passwords into the main project.",
                "Audit artifacts may keep ids, source ids, text_len, text_sha256, counts, freshness and permission scope."
            )
        }
        group_chat_evidence_contract = [pscustomobject]@{
            schema = "fb2.main_project.group_chat_evidence.v1"
            group_chat_test_method = "direct_api_read"
            screenshots_accepted = $false
            required_group_message_fields = @("message_id", "text_len", "text_sha256")
            required_visible_flow_evidence = @(
                "visible_mention_ai_reply_read",
                "selected_message_ai_reply_read",
                "summary_post_read",
                "feedback_quality_read"
            )
        }
        live_tool_manifest = [pscustomobject]@{
            status = "ready"
            tool_count = 3
            tool_ids = @("context_pack", "match_analysis_brief", "group_opinion_summary")
        }
        tool_result_envelope_contract = [pscustomobject]@{
            answer_source_validation = [pscustomobject]@{
                schema = "external_app.answer_source_validation.v1"
                rule = "records candidate/matched/unmatched source ids plus matched_tool_source_ids, allowed_tool_source_ids, has_missing_explicit_sources, and no_explicit_source_ids status"
            }
        }
    }
    $status = New-Fb2PublicContractStatus `
        -Base "http://example.invalid" `
        -Health "OK" `
        -Version ([pscustomobject]@{ versionName = "selftest"; gitSha = "abc123" }) `
        -Contract $syntheticContract

    $badContract = $syntheticContract.PSObject.Copy()
    $badContract.group_chat_evidence_contract = $badContract.group_chat_evidence_contract.PSObject.Copy()
    $badContract.group_chat_evidence_contract.screenshots_accepted = $true
    $badStatus = New-Fb2PublicContractStatus `
        -Base "http://example.invalid" `
        -Health "OK" `
        -Version ([pscustomobject]@{ versionName = "selftest"; gitSha = "abc123" }) `
        -Contract $badContract
    $badIndexContract = $syntheticContract | ConvertTo-Json -Depth 16 | ConvertFrom-Json
    $badIndexContract.domain_context_index_contract.indexes = @(
        [pscustomobject]@{ id = "match_index" },
        [pscustomobject]@{ id = "odds_snapshot_index" },
        [pscustomobject]@{ id = "current_user_ticket_index" },
        [pscustomobject]@{ id = "platform_order_risk_index" },
        [pscustomobject]@{ id = "group_opinion_index" },
        [pscustomobject]@{ id = "context_audit_index" },
        [pscustomobject]@{ id = "feedback_quality_index" },
        [pscustomobject]@{ id = "unrelated_extra_index" }
    )
    $badIndexContract.domain_context_index_contract.index_count = 8
    $badIndexStatus = New-Fb2PublicContractStatus `
        -Base "http://example.invalid" `
        -Health "OK" `
        -Version ([pscustomobject]@{ versionName = "selftest"; gitSha = "abc123" }) `
        -Contract $badIndexContract
    $badProjectionLayerContract = $syntheticContract | ConvertTo-Json -Depth 16 | ConvertFrom-Json
    $badProjectionLayerContract.context_projection_layer_contract.user_scenarios = @(
        $badProjectionLayerContract.context_projection_layer_contract.user_scenarios |
            Where-Object { $_.id -ne "group_discussion_summary_post" }
    )
    $badProjectionLayerContract.context_projection_layer_contract.user_scenario_count = 7
    $badProjectionLayerStatus = New-Fb2PublicContractStatus `
        -Base "http://example.invalid" `
        -Health "OK" `
        -Version ([pscustomobject]@{ versionName = "selftest"; gitSha = "abc123" }) `
        -Contract $badProjectionLayerContract
    $badToolResultContract = $syntheticContract | ConvertTo-Json -Depth 16 | ConvertFrom-Json
    $badToolResultContract.tool_result_envelope_contract.answer_source_validation.rule = "missing tool source audit ids"
    $badToolResultStatus = New-Fb2PublicContractStatus `
        -Base "http://example.invalid" `
        -Health "OK" `
        -Version ([pscustomobject]@{ versionName = "selftest"; gitSha = "abc123" }) `
        -Contract $badToolResultContract
    $badMissingSourceContract = $syntheticContract | ConvertTo-Json -Depth 16 | ConvertFrom-Json
    $badMissingSourceContract.tool_result_envelope_contract.answer_source_validation.rule = "records candidate/matched/unmatched source ids plus matched_tool_source_ids and allowed_tool_source_ids"
    $badMissingSourceStatus = New-Fb2PublicContractStatus `
        -Base "http://example.invalid" `
        -Health "OK" `
        -Version ([pscustomobject]@{ versionName = "selftest"; gitSha = "abc123" }) `
        -Contract $badMissingSourceContract
    $badRetrievalEvidenceContract = $syntheticContract | ConvertTo-Json -Depth 16 | ConvertFrom-Json
    $badRetrievalEvidenceContract.context_pack_template_contract.retrieval_evidence_item_shape.required_fields = @("source_id", "reason")
    $badRetrievalEvidenceStatus = New-Fb2PublicContractStatus `
        -Base "http://example.invalid" `
        -Health "OK" `
        -Version ([pscustomobject]@{ versionName = "selftest"; gitSha = "abc123" }) `
        -Contract $badRetrievalEvidenceContract
    $badQueryIntentContract = $syntheticContract | ConvertTo-Json -Depth 16 | ConvertFrom-Json
    $badQueryIntentContract.context_query_intent_contract.scenario_intents = @(
        $badQueryIntentContract.context_query_intent_contract.scenario_intents |
            Where-Object { $_.scenario_id -ne "my_ticket_analysis" }
    )
    $badQueryIntentStatus = New-Fb2PublicContractStatus `
        -Base "http://example.invalid" `
        -Health "OK" `
        -Version ([pscustomobject]@{ versionName = "selftest"; gitSha = "abc123" }) `
        -Contract $badQueryIntentContract

    $failed = 0
    if (-not [bool]$status.success) {
        Write-Output "FAIL`tpublic contract selftest valid status"
        $failed++
    } else {
        Write-Output "OK`tpublic contract selftest valid status"
    }
    if ([bool]$badStatus.success) {
        Write-Output "FAIL`tpublic contract selftest rejects screenshots"
        $failed++
    } else {
        Write-Output "OK`tpublic contract selftest rejects screenshots"
    }
    if ([bool]$badIndexStatus.success) {
        Write-Output "FAIL`tpublic contract selftest rejects missing domain index"
        $failed++
    } else {
        Write-Output "OK`tpublic contract selftest rejects missing domain index"
    }
    if ([bool]$badProjectionLayerStatus.success) {
        Write-Output "FAIL`tpublic contract selftest rejects missing projection layer scenario"
        $failed++
    } else {
        Write-Output "OK`tpublic contract selftest rejects missing projection layer scenario"
    }
    if ([bool]$badToolResultStatus.success) {
        Write-Output "FAIL`tpublic contract selftest rejects missing tool source audit"
        $failed++
    } else {
        Write-Output "OK`tpublic contract selftest rejects missing tool source audit"
    }
    if ([bool]$badMissingSourceStatus.success) {
        Write-Output "FAIL`tpublic contract selftest rejects missing no-source audit"
        $failed++
    } else {
        Write-Output "OK`tpublic contract selftest rejects missing no-source audit"
    }
    if ([bool]$badRetrievalEvidenceStatus.success) {
        Write-Output "FAIL`tpublic contract selftest rejects incomplete retrieval evidence shape"
        $failed++
    } else {
        Write-Output "OK`tpublic contract selftest rejects incomplete retrieval evidence shape"
    }
    if ([bool]$badQueryIntentStatus.success) {
        Write-Output "FAIL`tpublic contract selftest rejects incomplete query intent contract"
        $failed++
    } else {
        Write-Output "OK`tpublic contract selftest rejects incomplete query intent contract"
    }
    Write-Output "== SelfTest Summary =="
    Write-Output "failed=$failed"
    if ($failed -gt 0) {
        exit 1
    }
}

if ($SelfTest) {
    Invoke-Fb2PublicContractSelfTest
    return
}

Set-Fb2PublicDirectNetwork
$health = Invoke-Fb2PublicDirectRest -Uri "$MainBase/health" -TimeoutSec $RequestTimeoutSec
$version = Invoke-Fb2PublicDirectRest -Uri "$MainBase/api/server/version" -TimeoutSec $RequestTimeoutSec
$contract = Invoke-Fb2PublicDirectRest -Uri "$MainBase/api/external/apps/fb2/context-contract" -TimeoutSec $RequestTimeoutSec
$status = New-Fb2PublicContractStatus -Base $MainBase -Health $health -Version $version -Contract $contract

if ($OutputPath) {
    $parent = Split-Path -Parent $OutputPath
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $status | ConvertTo-Json -Depth 16 | Set-Content -Path $OutputPath -Encoding UTF8
}

$status | ConvertTo-Json -Depth 16
if (-not [bool]$status.success) {
    exit 1
}
