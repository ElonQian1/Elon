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
            domain_context_index_schema = ""
            domain_context_index_count = 0
            domain_context_index_ids = @()
            group_chat_test_method = ""
            screenshots_accepted = $false
            required_group_message_fields = @()
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
            domain_context_index_schema = ""
            domain_context_index_count = 0
            domain_context_index_ids = @()
            group_chat_test_method = ""
            screenshots_accepted = $false
            required_group_message_fields = @()
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
    $groupSchema = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "group_chat_evidence_schema")
    $groupMethod = ConvertTo-Fb2PublicContractSummaryText (Get-Fb2PublicContractSummaryProperty $summary "group_chat_test_method")
    $screenshotsRaw = Get-Fb2PublicContractSummaryProperty $summary "screenshots_accepted" $null
    $screenshotsAccepted = Test-Fb2PublicContractSummaryTruthy $screenshotsRaw
    $requiredFields = @((Get-Fb2PublicContractSummaryProperty $summary "required_group_message_fields" @()))
    $limitations = @((Get-Fb2PublicContractSummaryProperty $status "limitations" @()))
    $failedChecks = @((Get-Fb2PublicContractSummaryProperty $status "failed_checks" @()))

    $missing = @()
    if ($schema -ne "fb2.main_project.public_contract_status.v1") { $missing += "public_contract_status_schema" }
    if (-not $success) { $missing += "public_contract_status_success" }
    if ($domainSchema -ne "fb2.main_project.domain_data_blueprint.v1") { $missing += "domain_data_blueprint_contract" }
    if ($domainIndexSchema -ne "fb2.main_project.domain_context_index.v1") { $missing += "domain_context_index_contract" }
    if ($domainIndexCount -lt 8) { $missing += "domain_context_index_count" }
    foreach ($indexId in @("match_index", "current_user_ticket_index", "platform_order_risk_index", "group_opinion_index", "feedback_quality_index")) {
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
    if ($groupSchema -ne "fb2.main_project.group_chat_evidence.v1") { $missing += "group_chat_evidence_contract" }
    if ($groupMethod -ne "direct_api_read") { $missing += "group_chat_direct_api_read_contract" }
    if ($null -eq $screenshotsRaw -or $screenshotsAccepted) { $missing += "group_chat_rejects_screenshots_contract" }
    foreach ($field in @("message_id", "text_len", "text_sha256")) {
        if (-not ($requiredFields -contains $field)) {
            $missing += "group_chat_required_field_$field"
        }
    }
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
        context_pack_template_schema = $templateSchema
        context_pack_template_wrapper = $templateWrapper
        context_pack_template_sections = @($templateSections)
        domain_lane_count = [int](Get-Fb2PublicContractSummaryProperty $summary "domain_lane_count" 0)
        stores_fb2_business_data_in_main_project = Test-Fb2PublicContractSummaryTruthy (Get-Fb2PublicContractSummaryProperty $summary "stores_fb2_business_data_in_main_project")
        group_chat_evidence_schema = $groupSchema
        group_chat_test_method = $groupMethod
        screenshots_accepted = $screenshotsAccepted
        required_group_message_fields = @($requiredFields)
        live_tool_count = [int](Get-Fb2PublicContractSummaryProperty $summary "live_tool_count" 0)
        limitations = @($limitations)
        missing = @($missing | Select-Object -Unique)
    }
}
