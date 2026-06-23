#requires -Version 7.0

function Get-Fb2PublicContractSummaryProperty {
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

function Test-Fb2PublicContractSummaryTruthy {
    param([object]$Value)

    if ($null -eq $Value) {
        return $false
    }
    if ($Value -is [bool]) {
        return [bool]$Value
    }
    return ([string]$Value) -match "^(true|True|1)$"
}

function Test-Fb2PublicContractSummaryCheckPassed {
    param(
        [object]$Status,
        [string]$Id
    )

    foreach ($check in @((Get-Fb2PublicContractSummaryProperty $Status "checks" @()))) {
        if ([string](Get-Fb2PublicContractSummaryProperty $check "id" "") -eq $Id) {
            return Test-Fb2PublicContractSummaryTruthy (Get-Fb2PublicContractSummaryProperty $check "passed")
        }
    }
    return $false
}

function ConvertTo-Fb2PublicContractSummaryText {
    param([object]$Value)

    if ($null -eq $Value) {
        return ""
    }
    return [string]$Value
}

function Read-Fb2PublicContractSummaryJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return $null
    }
    try {
        return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    } catch {
        return $null
    }
}

function Get-Fb2PublicContractSummaryState {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return [ordered]@{
            path = ""
            exists = $false
            complete = $false
            schema = ""
            success = $false
            context_pack_template_schema = ""
            context_pack_template_wrapper = ""
            context_pack_template_sections = @()
            context_projection_layer_schema = ""
            context_projection_layer_lane_count = 0
            context_projection_layer_index_count = 0
            context_projection_layer_scenario_count = 0
            context_projection_layer_scenario_ids = @()
            domain_context_index_schema = ""
            domain_context_index_count = 0
            domain_context_index_ids = @()
            retrieval_evidence_item_shape_ready = $false
            context_pack_template_retrieval_evidence_schema = ""
            context_pack_template_retrieval_evidence_fields = @()
            domain_projection_retrieval_evidence_schema = ""
            domain_projection_retrieval_evidence_fields = @()
            context_projection_layer_retrieval_evidence_schema = ""
            context_projection_layer_retrieval_evidence_fields = @()
            domain_context_index_retrieval_evidence_schema = ""
            domain_context_index_retrieval_evidence_fields = @()
            context_query_intent_contract_ready = $false
            context_query_intent_schema = ""
            context_query_intent_complete = $false
            context_query_intent_scenario_count = 0
            context_query_intent_scenario_ids = @()
            context_query_intent_entrypoint_ids = @()
            context_query_intent_required_fields = @()
            group_chat_test_method = ""
            screenshots_accepted = $false
            required_group_message_fields = @()
            answer_source_validation_ready = $false
            answer_source_validation_schema = ""
            answer_source_validation_rule = ""
            answer_source_validation_schema_check = $false
            answer_source_validation_tool_sources_check = $false
            answer_source_validation_missing_sources_check = $false
            missing = @("public_contract_status_summary")
        }
    }

    $status = Read-Fb2PublicContractSummaryJson -Path $Path
    if ($null -eq $status) {
        return [ordered]@{
            path = $Path
            exists = $true
            complete = $false
            schema = ""
            success = $false
            context_pack_template_schema = ""
            context_pack_template_wrapper = ""
            context_pack_template_sections = @()
            context_projection_layer_schema = ""
            context_projection_layer_lane_count = 0
            context_projection_layer_index_count = 0
            context_projection_layer_scenario_count = 0
            context_projection_layer_scenario_ids = @()
            domain_context_index_schema = ""
            domain_context_index_count = 0
            domain_context_index_ids = @()
            retrieval_evidence_item_shape_ready = $false
            context_pack_template_retrieval_evidence_schema = ""
            context_pack_template_retrieval_evidence_fields = @()
            domain_projection_retrieval_evidence_schema = ""
            domain_projection_retrieval_evidence_fields = @()
            context_projection_layer_retrieval_evidence_schema = ""
            context_projection_layer_retrieval_evidence_fields = @()
            domain_context_index_retrieval_evidence_schema = ""
            domain_context_index_retrieval_evidence_fields = @()
            context_query_intent_contract_ready = $false
            context_query_intent_schema = ""
            context_query_intent_complete = $false
            context_query_intent_scenario_count = 0
            context_query_intent_scenario_ids = @()
            context_query_intent_entrypoint_ids = @()
            context_query_intent_required_fields = @()
            group_chat_test_method = ""
            screenshots_accepted = $false
            required_group_message_fields = @()
            answer_source_validation_ready = $false
            answer_source_validation_schema = ""
            answer_source_validation_rule = ""
            answer_source_validation_schema_check = $false
            answer_source_validation_tool_sources_check = $false
            answer_source_validation_missing_sources_check = $false
            missing = @("public_contract_status_summary_parse_error")
        }
    }

    $summary = Get-Fb2PublicContractSummaryProperty $status "contract_summary"
    $server = Get-Fb2PublicContractSummaryProperty $status "server"
    $schema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $status "schema")
    $success = Test-Fb2PublicContractSummaryTruthy (Get-Fb2PublicContractSummaryProperty $status "success")
    $domainSchema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "domain_data_blueprint_schema")
    $domainIndexSchema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "domain_context_index_schema")
    $domainIndexCount = [int](Get-Fb2PublicContractSummaryProperty $summary "domain_context_index_count" 0)
    $domainIndexIds = @((Get-Fb2PublicContractSummaryProperty $summary "domain_context_index_ids" @()))
    $templateSchema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "context_pack_template_schema")
    $templateWrapper = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "context_pack_template_wrapper")
    $templateSections = @((Get-Fb2PublicContractSummaryProperty $summary "context_pack_template_sections" @()))
    $templateRetrievalSchema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "context_pack_template_retrieval_evidence_schema")
    $templateRetrievalFields = @((Get-Fb2PublicContractSummaryProperty $summary "context_pack_template_retrieval_evidence_fields" @()))
    $domainProjectionRetrievalSchema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "domain_projection_retrieval_evidence_schema")
    $domainProjectionRetrievalFields = @((Get-Fb2PublicContractSummaryProperty $summary "domain_projection_retrieval_evidence_fields" @()))
    $projectionLayerRetrievalSchema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_retrieval_evidence_schema")
    $projectionLayerRetrievalFields = @((Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_retrieval_evidence_fields" @()))
    $domainIndexRetrievalSchema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "domain_context_index_retrieval_evidence_schema")
    $domainIndexRetrievalFields = @((Get-Fb2PublicContractSummaryProperty $summary "domain_context_index_retrieval_evidence_fields" @()))
    $queryIntentSchema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "context_query_intent_schema")
    $queryIntentComplete = Test-Fb2PublicContractSummaryTruthy (Get-Fb2PublicContractSummaryProperty $summary "context_query_intent_complete")
    $queryIntentScenarioCount = [int](Get-Fb2PublicContractSummaryProperty $summary "context_query_intent_scenario_count" 0)
    $queryIntentScenarioIds = @((Get-Fb2PublicContractSummaryProperty $summary "context_query_intent_scenario_ids" @()))
    $queryIntentEntrypointIds = @((Get-Fb2PublicContractSummaryProperty $summary "context_query_intent_entrypoint_ids" @()))
    $queryIntentRequiredFields = @((Get-Fb2PublicContractSummaryProperty $summary "context_query_intent_required_fields" @()))
    $projectionLayerSchema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_schema")
    $projectionLayerComplete = Test-Fb2PublicContractSummaryTruthy (Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_complete")
    $projectionLayerLaneCount = [int](Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_lane_count" 0)
    $projectionLayerLaneIds = @((Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_lane_ids" @()))
    $projectionLayerIndexCount = [int](Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_index_count" 0)
    $projectionLayerIndexIds = @((Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_index_ids" @()))
    $projectionLayerScenarioCount = [int](Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_scenario_count" 0)
    $projectionLayerScenarioIds = @((Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_scenario_ids" @()))
    $projectionLayerGroupMethod = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_group_method")
    $projectionLayerScreenshotsRaw = Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_screenshots_accepted" $null
    $projectionLayerScreenshotsAccepted = Test-Fb2PublicContractSummaryTruthy $projectionLayerScreenshotsRaw
    $projectionLayerGroupFields = @((Get-Fb2PublicContractSummaryProperty $summary "context_projection_layer_group_fields" @()))
    $groupSchema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "group_chat_evidence_schema")
    $groupMethod = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "group_chat_test_method")
    $screenshotsRaw = Get-Fb2PublicContractSummaryProperty $summary "screenshots_accepted" $null
    $screenshotsAccepted = Test-Fb2PublicContractSummaryTruthy $screenshotsRaw
    $requiredFields = @((Get-Fb2PublicContractSummaryProperty $summary "required_group_message_fields" @()))
    $answerSourceSchema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "answer_source_validation_schema")
    $answerSourceRule = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "answer_source_validation_rule")
    $answerSourceSchemaCheck = (
        (Test-Fb2PublicContractSummaryCheckPassed -Status $status -Id "tool_result_answer_source_validation_schema") `
            -or $answerSourceSchema -eq "external_app.answer_source_validation.v1"
    )
    $answerSourceToolSourcesCheck = (
        (Test-Fb2PublicContractSummaryCheckPassed -Status $status -Id "tool_result_answer_source_validation_tool_sources") `
            -or $answerSourceRule -match "matched_tool_source_ids"
    )
    $answerSourceMissingSourcesCheck = (
        (Test-Fb2PublicContractSummaryCheckPassed -Status $status -Id "tool_result_answer_source_validation_missing_sources") `
            -or ($answerSourceRule -match "has_missing_explicit_sources" -and $answerSourceRule -match "no_explicit_source_ids")
    )
    if ([string]::IsNullOrWhiteSpace($answerSourceSchema) -and $answerSourceSchemaCheck) {
        $answerSourceSchema = "external_app.answer_source_validation.v1"
    }
    if ([string]::IsNullOrWhiteSpace($answerSourceRule) -and $answerSourceToolSourcesCheck) {
        $answerSourceRule = "matched_tool_source_ids required by public contract checks"
    }
    if ([string]::IsNullOrWhiteSpace($answerSourceRule) -and $answerSourceMissingSourcesCheck) {
        $answerSourceRule = "has_missing_explicit_sources and no_explicit_source_ids required by public contract checks"
    }
    $answerSourceReady = ($answerSourceSchemaCheck -and $answerSourceToolSourcesCheck -and $answerSourceMissingSourcesCheck)
    $retrievalEvidenceRequiredFields = @("source_id", "source_kind", "lane_id", "index_id", "reason", "freshness", "permission_scope", "citation_source_id")
    $retrievalEvidenceShapeReady = (
        $templateRetrievalSchema -eq "fb2.retrieval_evidence_item.v1" `
            -and $domainProjectionRetrievalSchema -eq "fb2.retrieval_evidence_item.v1" `
            -and $projectionLayerRetrievalSchema -eq "fb2.retrieval_evidence_item.v1" `
            -and $domainIndexRetrievalSchema -eq "fb2.retrieval_evidence_item.v1"
    )
    foreach ($field in $retrievalEvidenceRequiredFields) {
        if (-not ($templateRetrievalFields -contains $field)) { $retrievalEvidenceShapeReady = $false }
        if (-not ($domainProjectionRetrievalFields -contains $field)) { $retrievalEvidenceShapeReady = $false }
        if (-not ($projectionLayerRetrievalFields -contains $field)) { $retrievalEvidenceShapeReady = $false }
        if (-not ($domainIndexRetrievalFields -contains $field)) { $retrievalEvidenceShapeReady = $false }
    }
    $queryIntentReady = (
        $queryIntentSchema -eq "fb2.context_query_intent.v1" `
            -and $queryIntentComplete `
            -and $queryIntentScenarioCount -ge 7
    )
    foreach ($field in @("query_intent_id", "entrypoint", "scenario_id", "group_id", "topic_hint", "intent_lanes", "requested_indexes", "permission_scope", "source_request", "output_limits")) {
        if (-not ($queryIntentRequiredFields -contains $field)) { $queryIntentReady = $false }
    }
    foreach ($entrypointId in @("group_mention_at_el", "selected_message_ai_reply", "group_summary_post", "chat_bootstrap_ai_reply")) {
        if (-not ($queryIntentEntrypointIds -contains $entrypointId)) { $queryIntentReady = $false }
    }
    foreach ($scenarioId in @("today_matches_analysis", "my_ticket_analysis", "platform_order_risk", "group_opinion_summary", "selected_message_review", "group_discussion_summary_post", "source_reference_audit")) {
        if (-not ($queryIntentScenarioIds -contains $scenarioId)) { $queryIntentReady = $false }
    }
    $limitations = @((Get-Fb2PublicContractSummaryProperty $status "limitations" @()))
    $failedChecks = @((Get-Fb2PublicContractSummaryProperty $status "failed_checks" @()))

    $missing = @()
    if ($schema -ne "fb2.main_project.public_contract_status.v1") { $missing += "public_contract_status_schema" }
    if (-not $success) { $missing += "public_contract_status_success" }
    if ($domainSchema -ne "fb2.main_project.domain_data_blueprint.v1") { $missing += "domain_data_blueprint_contract" }
    if ($domainIndexSchema -ne "fb2.main_project.domain_context_index.v1") { $missing += "domain_context_index_contract" }
    if ($domainIndexCount -lt 8) { $missing += "domain_context_index_count" }
    foreach ($indexId in @(
            "match_index",
            "odds_snapshot_index",
            "current_user_ticket_index",
            "platform_order_risk_index",
            "group_opinion_index",
            "opinion_memory_index",
            "context_audit_index",
            "feedback_quality_index"
        )) {
        if (-not ($domainIndexIds -contains $indexId)) {
            $missing += "domain_context_index_$indexId"
        }
    }
    if ($templateSchema -ne "fb2.context_pack_template.v1") { $missing += "context_pack_template_contract" }
    if ($templateWrapper -ne "fb2_context_pack") { $missing += "context_pack_template_wrapper" }
    foreach ($section in @("user_order_slice", "group_opinion_slice", "retrieval_evidence", "quality_feedback")) {
        if (-not ($templateSections -contains $section)) {
            $missing += "context_pack_template_section_$section"
        }
    }
    if ($projectionLayerSchema -ne "fb2.main_project.context_projection_layer.v1") { $missing += "context_projection_layer_contract" }
    if (-not $projectionLayerComplete) { $missing += "context_projection_layer_complete" }
    if ($projectionLayerLaneCount -lt 6) { $missing += "context_projection_layer_lane_count" }
    foreach ($laneId in @("match_facts_and_odds", "current_user_tickets", "platform_order_summary", "group_opinions", "opinion_learning_loop", "quality_feedback_audit")) {
        if (-not ($projectionLayerLaneIds -contains $laneId)) {
            $missing += "context_projection_layer_lane_$laneId"
        }
    }
    if ($projectionLayerIndexCount -lt 8) { $missing += "context_projection_layer_index_count" }
    foreach ($indexId in @("match_index", "odds_snapshot_index", "current_user_ticket_index", "platform_order_risk_index", "group_opinion_index", "opinion_memory_index", "context_audit_index", "feedback_quality_index")) {
        if (-not ($projectionLayerIndexIds -contains $indexId)) {
            $missing += "context_projection_layer_index_$indexId"
        }
    }
    if ($projectionLayerScenarioCount -lt 7) { $missing += "context_projection_layer_scenario_count" }
    foreach ($scenarioId in @("today_matches_analysis", "my_ticket_analysis", "platform_order_risk", "group_opinion_summary", "selected_message_review", "group_discussion_summary_post", "source_reference_audit")) {
        if (-not ($projectionLayerScenarioIds -contains $scenarioId)) {
            $missing += "context_projection_layer_scenario_$scenarioId"
        }
    }
    if ($projectionLayerGroupMethod -ne "direct_api_read") { $missing += "context_projection_layer_group_direct_api_read" }
    if ($null -eq $projectionLayerScreenshotsRaw -or $projectionLayerScreenshotsAccepted) { $missing += "context_projection_layer_rejects_screenshots" }
    if (-not ($projectionLayerGroupFields -contains "text_sha256")) { $missing += "context_projection_layer_group_field_text_sha256" }
    if ($groupSchema -ne "fb2.main_project.group_chat_evidence.v1") { $missing += "group_chat_evidence_contract" }
    if ($groupMethod -ne "direct_api_read") { $missing += "group_chat_direct_api_read_contract" }
    if ($null -eq $screenshotsRaw -or $screenshotsAccepted) { $missing += "group_chat_rejects_screenshots_contract" }
    foreach ($field in @("message_id", "text_len", "text_sha256")) {
        if (-not ($requiredFields -contains $field)) {
            $missing += "group_chat_required_field_$field"
        }
    }
    if (-not $answerSourceSchemaCheck) { $missing += "answer_source_validation_schema" }
    if (-not $answerSourceToolSourcesCheck) { $missing += "answer_source_validation_tool_sources" }
    if (-not $answerSourceMissingSourcesCheck) { $missing += "answer_source_validation_missing_sources" }
    if (-not $retrievalEvidenceShapeReady) { $missing += "retrieval_evidence_item_shape" }
    if (-not $queryIntentReady) { $missing += "context_query_intent_contract" }
    if (-not ($limitations -contains "does_not_verify_fb2_live_context_pack_or_orders")) {
        $missing += "public_contract_limitations_live_data_boundary"
    }

    [ordered]@{
        path = $Path
        exists = $true
        complete = (@($missing).Count -eq 0)
        schema = $schema
        main_base = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $status "main_base")
        server_version = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $server "versionName")
        server_git_sha = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $server "gitSha")
        success = $success
        passed_count = [int](Get-Fb2PublicContractSummaryProperty $status "passed_count" 0)
        failed_count = [int](Get-Fb2PublicContractSummaryProperty $status "failed_count" 0)
        failed_checks = @($failedChecks)
        domain_data_blueprint_schema = $domainSchema
        domain_context_index_schema = $domainIndexSchema
        domain_context_index_count = $domainIndexCount
        domain_context_index_ids = @($domainIndexIds)
        retrieval_evidence_item_shape_ready = $retrievalEvidenceShapeReady
        context_pack_template_retrieval_evidence_schema = $templateRetrievalSchema
        context_pack_template_retrieval_evidence_fields = @($templateRetrievalFields)
        domain_projection_retrieval_evidence_schema = $domainProjectionRetrievalSchema
        domain_projection_retrieval_evidence_fields = @($domainProjectionRetrievalFields)
        context_projection_layer_retrieval_evidence_schema = $projectionLayerRetrievalSchema
        context_projection_layer_retrieval_evidence_fields = @($projectionLayerRetrievalFields)
        domain_context_index_retrieval_evidence_schema = $domainIndexRetrievalSchema
        domain_context_index_retrieval_evidence_fields = @($domainIndexRetrievalFields)
        context_query_intent_contract_ready = $queryIntentReady
        context_query_intent_schema = $queryIntentSchema
        context_query_intent_complete = $queryIntentComplete
        context_query_intent_scenario_count = $queryIntentScenarioCount
        context_query_intent_scenario_ids = @($queryIntentScenarioIds)
        context_query_intent_entrypoint_ids = @($queryIntentEntrypointIds)
        context_query_intent_required_fields = @($queryIntentRequiredFields)
        context_pack_template_schema = $templateSchema
        context_pack_template_wrapper = $templateWrapper
        context_pack_template_sections = @($templateSections)
        context_projection_layer_schema = $projectionLayerSchema
        context_projection_layer_lane_count = $projectionLayerLaneCount
        context_projection_layer_lane_ids = @($projectionLayerLaneIds)
        context_projection_layer_index_count = $projectionLayerIndexCount
        context_projection_layer_index_ids = @($projectionLayerIndexIds)
        context_projection_layer_scenario_count = $projectionLayerScenarioCount
        context_projection_layer_scenario_ids = @($projectionLayerScenarioIds)
        context_projection_layer_group_method = $projectionLayerGroupMethod
        context_projection_layer_screenshots_accepted = $projectionLayerScreenshotsAccepted
        context_projection_layer_group_fields = @($projectionLayerGroupFields)
        domain_lane_count = [int](Get-Fb2PublicContractSummaryProperty $summary "domain_lane_count" 0)
        stores_fb2_business_data_in_main_project = Test-Fb2PublicContractSummaryTruthy (Get-Fb2PublicContractSummaryProperty $summary "stores_fb2_business_data_in_main_project")
        group_chat_evidence_schema = $groupSchema
        group_chat_test_method = $groupMethod
        screenshots_accepted = $screenshotsAccepted
        required_group_message_fields = @($requiredFields)
        live_tool_count = [int](Get-Fb2PublicContractSummaryProperty $summary "live_tool_count" 0)
        answer_source_validation_ready = $answerSourceReady
        answer_source_validation_schema = $answerSourceSchema
        answer_source_validation_rule = $answerSourceRule
        answer_source_validation_schema_check = $answerSourceSchemaCheck
        answer_source_validation_tool_sources_check = $answerSourceToolSourcesCheck
        answer_source_validation_missing_sources_check = $answerSourceMissingSourcesCheck
        limitations = @($limitations)
        missing = @($missing | Select-Object -Unique)
    }
}
