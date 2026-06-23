#requires -Version 7.0

param(
    [string]$StatusPath = "",
    [string]$OutputPath = "",
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-Fb2ScenarioRepoRoot {
    Split-Path -Parent $PSScriptRoot
}

function Resolve-Fb2ScenarioPath {
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

function Get-Fb2ScenarioProperty {
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
    if ($Object -is [System.Collections.Specialized.OrderedDictionary] -and $Object.Contains($Name)) {
        return $Object[$Name]
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $Default
    }
    return $property.Value
}

function Read-Fb2ScenarioJson {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        throw "Status not found: $Path. Run scripts\smoke-fb2-ai-center-status.ps1 or scripts\fb2-ai-center-refresh-current-status.ps1 first."
    }
    Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Add-Fb2ScenarioCheck {
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

function Test-Fb2ScenarioContainsAll {
    param(
        [object[]]$Values,
        [string[]]$Required
    )

    $actual = @($Values | ForEach-Object { [string]$_ })
    foreach ($item in $Required) {
        if (-not ($actual -contains $item)) {
            return $false
        }
    }
    return $true
}

function Test-Fb2ScenarioSecretSafe {
    param([string]$Text)

    if ([string]::IsNullOrWhiteSpace($Text)) {
        return $true
    }
    if ($Text -match '(?i)FB2_AI_CENTER_TOKEN\s*=\s*["''][^<]') {
        return $false
    }
    if ($Text -match '(?i)-Fb2(AiCenter)?Token\s+(?!<FB2_AI_CENTER_TOKEN>)[^\s]+') {
        return $false
    }
    if ($Text -match '(?i)-Fb2Password\s+(?!<FB2_PASSWORD>)[^\s]+') {
        return $false
    }
    if ($Text -match '(?i)(bearer|token|password|secret)[=:]\s*(?!<)[A-Za-z0-9_\-\.]{12,}') {
        return $false
    }
    return $true
}

function Test-Fb2ScenarioNoRawBody {
    param([object]$Evidence)

    if ($null -eq $Evidence) {
        return $true
    }
    $json = $Evidence | ConvertTo-Json -Depth 8 -Compress
    return -not ($json -match '(?i)(raw_text|full_text|message_body|order_body|content=|body=|\btext=|"text"\s*:)')
}

function Find-Fb2ScenarioItem {
    param(
        [object[]]$Scenarios,
        [string]$Id
    )

    @($Scenarios | Where-Object { [string](Get-Fb2ScenarioProperty $_ "id" "") -eq $Id } | Select-Object -First 1)
}

function Test-Fb2ScenarioEvidenceHash {
    param([string]$Value)

    -not [string]::IsNullOrWhiteSpace($Value) -and $Value -match '^[a-fA-F0-9]{64}$'
}

function Test-Fb2ScenarioDirectReadLine {
    param(
        [string]$Value,
        [string]$Kind
    )

    if ([string]::IsNullOrWhiteSpace($Value)) {
        return $false
    }
    $hasId = if ($Kind -eq "post") {
        $Value -match '\bpost=[A-Za-z0-9_\-]+'
    } else {
        $Value -match '\bmessage=[A-Za-z0-9_\-]+'
    }
    return (
        $Value -match '\bgroup=[A-Za-z0-9_\-]+' -and
        $hasId -and
        $Value -match '\btext_len=\d+' -and
        $Value -match '\btext_sha256=[a-fA-F0-9]{64}'
    )
}

function Test-Fb2ScenarioZeroValue {
    param([object]$Value)

    $text = [string]$Value
    if ([string]::IsNullOrWhiteSpace($text)) {
        return $false
    }
    return $text -match '\b(value|count)=0\b|^0$'
}

function New-Fb2ScenarioContract {
    @(
        [pscustomobject][ordered]@{
            id = "today_matches_analysis"
            evidence_mode = "offline_context_pack_sample_source_coverage"
            sources = @("match", "odds", "context_audit")
            layers = @("match_facts", "odds_facts", "ai_inference", "risk_boundary")
            forbidden = @("guaranteed_win", "fabricated_odds")
            evidence_kind = "context_pack"
        },
        [pscustomobject][ordered]@{
            id = "my_ticket_analysis"
            evidence_mode = "offline_context_pack_sample_source_coverage"
            sources = @("user_order", "ticket", "context_audit")
            layers = @("match_facts", "current_user_orders", "ai_inference", "risk_boundary")
            forbidden = @("other_user_order_detail", "guaranteed_win")
            evidence_kind = "context_pack"
        },
        [pscustomobject][ordered]@{
            id = "platform_order_risk"
            evidence_mode = "offline_context_pack_sample_source_coverage"
            sources = @("platform_order_summary", "context_audit")
            layers = @("platform_aggregate", "ai_inference", "risk_boundary")
            forbidden = @("single_user_order_detail", "user_identity_leak")
            evidence_kind = "context_pack"
        },
        [pscustomobject][ordered]@{
            id = "group_opinion_summary"
            evidence_mode = "offline_context_pack_sample_source_coverage"
            sources = @("group_message", "opinion_memory", "context_audit")
            layers = @("group_opinion", "match_facts", "ai_inference", "risk_boundary")
            forbidden = @("group_opinion_as_fact", "fabricated_group_view")
            evidence_kind = "context_pack"
        },
        [pscustomobject][ordered]@{
            id = "selected_message_review"
            evidence_mode = "visible_group_direct_read_evidence"
            sources = @("selected_message", "match", "context_audit")
            layers = @("selected_message_fact", "match_facts", "ai_inference", "risk_boundary")
            forbidden = @("selected_message_as_external_fact", "guaranteed_win")
            evidence_kind = "selected_message"
        },
        [pscustomobject][ordered]@{
            id = "group_discussion_summary_post"
            evidence_mode = "visible_summary_post_direct_read_and_feedback"
            sources = @("group_message", "opinion_memory", "context_audit")
            layers = @("group_opinion", "match_facts", "ai_inference", "risk_boundary")
            forbidden = @("fabricated_group_view", "guaranteed_win")
            evidence_kind = "summary_post"
        },
        [pscustomobject][ordered]@{
            id = "source_reference_audit"
            evidence_mode = "context_projection_and_quality_summary"
            sources = @("context_audit", "match", "group_message")
            layers = @("source_registry", "data_fact_boundary", "quality_feedback")
            forbidden = @("uncited_source", "fabricated_source")
            evidence_kind = "source_audit"
        }
    )
}

function Add-Fb2ScenarioEvidenceChecks {
    param(
        [System.Collections.ArrayList]$Checks,
        [object]$Scenario,
        [string]$Id,
        [string]$Kind
    )

    $evidence = Get-Fb2ScenarioProperty $Scenario "evidence"
    Add-Fb2ScenarioCheck $Checks "$Id evidence no raw body" (Test-Fb2ScenarioNoRawBody -Evidence $evidence)
    Add-Fb2ScenarioCheck $Checks "$Id evidence secret safe" (Test-Fb2ScenarioSecretSafe -Text ($evidence | ConvertTo-Json -Depth 8 -Compress))

    if ($Kind -eq "context_pack") {
        Add-Fb2ScenarioCheck $Checks "$Id context audit id present" (-not [string]::IsNullOrWhiteSpace([string](Get-Fb2ScenarioProperty $evidence "context_audit_id" "")))
        Add-Fb2ScenarioCheck $Checks "$Id citation source count positive" ([int](Get-Fb2ScenarioProperty $evidence "citation_source_count" 0) -gt 0)
        Add-Fb2ScenarioCheck $Checks "$Id context pack sha256" (Test-Fb2ScenarioEvidenceHash -Value ([string](Get-Fb2ScenarioProperty $evidence "context_pack_sha256" "")))
        return
    }

    if ($Kind -eq "selected_message") {
        Add-Fb2ScenarioCheck $Checks "$Id seed hash line" (Test-Fb2ScenarioDirectReadLine -Value ([string](Get-Fb2ScenarioProperty $evidence "selected_message_seed" "")) -Kind "message")
        Add-Fb2ScenarioCheck $Checks "$Id reply hash line" (Test-Fb2ScenarioDirectReadLine -Value ([string](Get-Fb2ScenarioProperty $evidence "selected_message_reply" "")) -Kind "message")
        return
    }

    if ($Kind -eq "summary_post") {
        Add-Fb2ScenarioCheck $Checks "$Id summary post hash line" (Test-Fb2ScenarioDirectReadLine -Value ([string](Get-Fb2ScenarioProperty $evidence "summary_post" "")) -Kind "post")
        Add-Fb2ScenarioCheck $Checks "$Id feedback complete" ([bool](Get-Fb2ScenarioProperty $evidence "feedback_complete" $false))
        return
    }

    if ($Kind -eq "source_audit") {
        Add-Fb2ScenarioCheck $Checks "$Id context projection complete" ([bool](Get-Fb2ScenarioProperty $evidence "context_projection_complete" $false))
        Add-Fb2ScenarioCheck $Checks "$Id unmatched cited sources zero" (Test-Fb2ScenarioZeroValue -Value (Get-Fb2ScenarioProperty $evidence "quality_unmatched_cited_sources" ""))
    }
}

function New-Fb2ScenarioAuditValidation {
    param(
        [object]$Status,
        [string]$SourcePath
    )

    $checks = [System.Collections.ArrayList]::new()
    $audit = Get-Fb2ScenarioProperty $Status "latest_user_scenario_audit"
    $scenarios = @(Get-Fb2ScenarioProperty $audit "scenarios" @())
    $missing = @(Get-Fb2ScenarioProperty $audit "missing" @())
    $contract = New-Fb2ScenarioContract

    Add-Fb2ScenarioCheck $checks "audit present" ($null -ne $audit)
    Add-Fb2ScenarioCheck $checks "audit schema" ([string](Get-Fb2ScenarioProperty $audit "schema" "") -eq "fb2.main_project.user_scenario_audit.v1")
    Add-Fb2ScenarioCheck $checks "context format" ([string](Get-Fb2ScenarioProperty $audit "context_format" "") -eq "xml_wrapped_markdown_context_pack_with_json_metadata")
    Add-Fb2ScenarioCheck $checks "mcp first phase boundary" ([string](Get-Fb2ScenarioProperty $audit "mcp_status" "") -match "not_first_phase|rest_context_pack|tool_manifest")
    Add-Fb2ScenarioCheck $checks "scenario count declared" ([int](Get-Fb2ScenarioProperty $audit "scenario_count" 0) -eq 7)
    Add-Fb2ScenarioCheck $checks "scenario count actual" (@($scenarios).Count -eq 7)
    Add-Fb2ScenarioCheck $checks "complete count" ([int](Get-Fb2ScenarioProperty $audit "complete_count" 0) -eq 7)
    Add-Fb2ScenarioCheck $checks "failed count zero" ([int](Get-Fb2ScenarioProperty $audit "failed_count" -1) -eq 0)
    Add-Fb2ScenarioCheck $checks "audit complete" ([bool](Get-Fb2ScenarioProperty $audit "complete" $false))
    Add-Fb2ScenarioCheck $checks "audit missing empty" (@($missing).Count -eq 0)

    $ids = @($scenarios | ForEach-Object { [string](Get-Fb2ScenarioProperty $_ "id" "") })
    Add-Fb2ScenarioCheck $checks "no duplicate scenario ids" (@($ids | Group-Object | Where-Object { $_.Count -gt 1 }).Count -eq 0)

    foreach ($expected in $contract) {
        $id = [string]$expected.id
        $item = Find-Fb2ScenarioItem -Scenarios $scenarios -Id $id
        Add-Fb2ScenarioCheck $checks "has scenario $id" (@($item).Count -gt 0)
        if (@($item).Count -eq 0) {
            continue
        }
        $scenario = $item[0]
        Add-Fb2ScenarioCheck $checks "$id complete" ([bool](Get-Fb2ScenarioProperty $scenario "complete" $false))
        Add-Fb2ScenarioCheck $checks "$id missing empty" (@(Get-Fb2ScenarioProperty $scenario "missing" @()).Count -eq 0)
        Add-Fb2ScenarioCheck $checks "$id evidence mode" ([string](Get-Fb2ScenarioProperty $scenario "evidence_mode" "") -eq [string]$expected.evidence_mode)
        Add-Fb2ScenarioCheck $checks "$id source kinds" (Test-Fb2ScenarioContainsAll -Values @(Get-Fb2ScenarioProperty $scenario "required_source_kinds" @()) -Required @($expected.sources))
        Add-Fb2ScenarioCheck $checks "$id answer layers" (Test-Fb2ScenarioContainsAll -Values @(Get-Fb2ScenarioProperty $scenario "required_answer_layers" @()) -Required @($expected.layers))
        Add-Fb2ScenarioCheck $checks "$id forbidden outputs" (Test-Fb2ScenarioContainsAll -Values @(Get-Fb2ScenarioProperty $scenario "forbidden_outputs" @()) -Required @($expected.forbidden))
        Add-Fb2ScenarioEvidenceChecks -Checks $checks -Scenario $scenario -Id $id -Kind ([string]$expected.evidence_kind)
    }

    $failed = @($checks | Where-Object { -not [bool]$_.passed })
    [ordered]@{
        schema = "fb2.main_project.user_scenario_audit_validation.v1"
        source_status = $SourcePath
        success = (@($failed).Count -eq 0)
        check_count = @($checks).Count
        failed_count = @($failed).Count
        failed = @($failed)
        checks = @($checks)
        scenario_count = @($scenarios).Count
        required_scenarios = @($contract | ForEach-Object { [string]$_.id })
        note = "Validates the product-level fb2 user question scenarios without reading or storing order/message bodies."
    }
}

function New-Fb2ScenarioFixtureScenario {
    param(
        [string]$Id,
        [string]$Mode,
        [string[]]$Sources,
        [string[]]$Layers,
        [string[]]$Forbidden,
        [object]$Evidence
    )

    [pscustomobject][ordered]@{
        id = $Id
        user_question = $Id
        evidence_mode = $Mode
        complete = $true
        required_source_kinds = @($Sources)
        required_answer_layers = @($Layers)
        forbidden_outputs = @($Forbidden)
        missing = @()
        evidence = $Evidence
    }
}

function New-Fb2ScenarioFixtureStatus {
    $hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    $contract = New-Fb2ScenarioContract
    $scenarios = @(
        foreach ($expected in $contract) {
            $evidence = switch ([string]$expected.evidence_kind) {
                "context_pack" { [ordered]@{ context_audit_id = "audit-$($expected.id)"; citation_source_count = 3; context_pack_sha256 = $hash } }
                "selected_message" {
                    [ordered]@{
                        selected_message_seed = "group=ext_fb2_official message=gmsg_1 sender=usr_1 created_at=2026-06-23T00:00:00Z text_len=20 text_sha256=$hash"
                        selected_message_reply = "group=ext_fb2_official message=gai_1 sender=usr_elon_ai created_at=2026-06-23T00:00:01Z text_len=40 text_sha256=$hash"
                    }
                }
                "summary_post" { [ordered]@{ summary_post = "group=ext_fb2_official post=gsp_1 status=ready text_len=80 text_sha256=$hash"; summary_post_ready_for_mode = $false; feedback_complete = $true } }
                "source_audit" { [ordered]@{ context_projection_complete = $true; quality_unmatched_cited_sources = "value=0" } }
            }
            New-Fb2ScenarioFixtureScenario `
                -Id ([string]$expected.id) `
                -Mode ([string]$expected.evidence_mode) `
                -Sources @($expected.sources) `
                -Layers @($expected.layers) `
                -Forbidden @($expected.forbidden) `
                -Evidence $evidence
        }
    )
    [pscustomobject]@{
        latest_user_scenario_audit = [ordered]@{
            schema = "fb2.main_project.user_scenario_audit.v1"
            context_format = "xml_wrapped_markdown_context_pack_with_json_metadata"
            mcp_status = "not_first_phase_use_rest_context_pack_and_tool_manifest_first"
            scenario_count = 7
            complete_count = 7
            failed_count = 0
            complete = $true
            scenarios = @($scenarios)
            missing = @()
        }
    }
}

function Invoke-Fb2ScenarioSelfTest {
    $failed = 0
    $good = New-Fb2ScenarioFixtureStatus
    $goodResult = New-Fb2ScenarioAuditValidation -Status $good -SourcePath "selftest-good.json"
    if (-not [bool]$goodResult.success) {
        $goodResult | ConvertTo-Json -Depth 8
        $failed++
    }

    $cases = @(
        @{ name = "missing-scenario"; edit = { param($s) $s.latest_user_scenario_audit.scenarios = @($s.latest_user_scenario_audit.scenarios | Where-Object { [string]$_.id -ne "my_ticket_analysis" }) } },
        @{ name = "missing-forbidden"; edit = { param($s) (@($s.latest_user_scenario_audit.scenarios | Where-Object { [string]$_.id -eq "today_matches_analysis" })[0]).forbidden_outputs = @("guaranteed_win") } },
        @{ name = "raw-selected-text"; edit = { param($s) (@($s.latest_user_scenario_audit.scenarios | Where-Object { [string]$_.id -eq "selected_message_review" })[0]).evidence.selected_message_seed = "group=ext_fb2_official message=gmsg_1 text=raw body text_sha256=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" } },
        @{ name = "unmatched-sources"; edit = { param($s) (@($s.latest_user_scenario_audit.scenarios | Where-Object { [string]$_.id -eq "source_reference_audit" })[0]).evidence.quality_unmatched_cited_sources = "value=2" } },
        @{ name = "mcp-first"; edit = { param($s) $s.latest_user_scenario_audit.mcp_status = "mcp_is_first_phase_fact_source" } }
    )

    foreach ($case in $cases) {
        $fixture = $good | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        & $case.edit $fixture
        $result = New-Fb2ScenarioAuditValidation -Status $fixture -SourcePath ("selftest-" + [string]$case.name + ".json")
        if ([bool]$result.success) {
            $failed++
        }
    }

    Write-Output "== SelfTest Summary =="
    Write-Output "failed=$failed"
    if ($failed -gt 0) {
        exit 1
    }
}

if ($SelfTest) {
    Invoke-Fb2ScenarioSelfTest
    exit 0
}

$root = Get-Fb2ScenarioRepoRoot
if ([string]::IsNullOrWhiteSpace($StatusPath)) {
    $StatusPath = Join-Path $root "target\fb2-ai-center\status-current.json"
} else {
    $StatusPath = Resolve-Fb2ScenarioPath -Path $StatusPath -Root $root
}
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $root "target\fb2-ai-center\user-scenario-audit-validation-current.json"
} else {
    $OutputPath = Resolve-Fb2ScenarioPath -Path $OutputPath -Root $root
}

$parent = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($parent)) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

$status = Read-Fb2ScenarioJson -Path $StatusPath
$result = New-Fb2ScenarioAuditValidation -Status $status -SourcePath $StatusPath
$json = $result | ConvertTo-Json -Depth 8
Set-Content -LiteralPath $OutputPath -Value $json -Encoding UTF8
$json

if (-not [bool]$result.success) {
    exit 1
}
