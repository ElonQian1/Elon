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
    $group = $Contract.group_chat_evidence_contract
    $manifest = $Contract.live_tool_manifest
    $domainLaneIds = @($domain.lanes | ForEach-Object { $_.id })
    $domainSections = @($domain.required_context_pack_sections)
    $domainMetadata = @($domain.required_metadata)
    $domainAntiPatterns = @($domain.anti_patterns)
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
