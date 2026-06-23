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
    $domainIndex = $Contract.domain_context_index_contract
    $template = $Contract.context_pack_template_contract
    $group = $Contract.group_chat_evidence_contract
    $manifest = $Contract.live_tool_manifest
    $templateSections = @($template.required_section_order)
    $templateMetadata = @($template.required_metadata)
    $templateSourceKinds = @($template.citation_source_shape.business_source_kinds)
    $domainLaneIds = @($domain.lanes | ForEach-Object { $_.id })
    $domainSections = @($domain.required_context_pack_sections)
    $domainMetadata = @($domain.required_metadata)
    $domainAntiPatterns = @($domain.anti_patterns)
    $domainIndexIds = @($domainIndex.indexes | ForEach-Object { $_.id })
    $domainIndexInputs = @($domainIndex.required_query_inputs)
    $domainIndexMetrics = @($domainIndex.required_metrics)
    $domainIndexNotAllowed = @($domainIndex.index_output_boundary.not_allowed)
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
            domain_data_blueprint_schema = [string]$Contract.domain_data_blueprint_contract.schema
            domain_context_index_schema = [string]$Contract.domain_context_index_contract.schema
            domain_context_index_count = [int]$Contract.domain_context_index_contract.index_count
            domain_context_index_ids = @($Contract.domain_context_index_contract.indexes | ForEach-Object { $_.id })
            context_pack_template_schema = [string]$Contract.context_pack_template_contract.schema
            context_pack_template_wrapper = [string]$Contract.context_pack_template_contract.body.wrapper
            context_pack_template_sections = @($Contract.context_pack_template_contract.required_section_order)
            domain_lane_count = [int]$Contract.domain_data_blueprint_contract.lane_count
            stores_fb2_business_data_in_main_project = [bool]$Contract.domain_data_blueprint_contract.stores_fb2_business_data_in_main_project
            group_chat_evidence_schema = [string]$Contract.group_chat_evidence_contract.schema
            group_chat_test_method = [string]$Contract.group_chat_evidence_contract.group_chat_test_method
            screenshots_accepted = [bool]$Contract.group_chat_evidence_contract.screenshots_accepted
            required_group_message_fields = @($Contract.group_chat_evidence_contract.required_group_message_fields)
            live_tool_count = [int]$Contract.live_tool_manifest.tool_count
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

$health = Invoke-RestMethod -Method Get -Uri "$MainBase/health" -TimeoutSec $RequestTimeoutSec
$version = Invoke-RestMethod -Method Get -Uri "$MainBase/api/server/version" -TimeoutSec $RequestTimeoutSec
$contract = Invoke-RestMethod -Method Get -Uri "$MainBase/api/external/apps/fb2/context-contract" -TimeoutSec $RequestTimeoutSec
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
