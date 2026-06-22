#requires -Version 7.0

function Get-Fb2ContextProjectionText {
    param([object]$Data)

    if ($null -eq $Data) {
        return ""
    }
    return [string]$Data.context_pack
}

function Test-Fb2ProjectionTextContainsAny {
    param(
        [string]$Text,
        [string[]]$Needles
    )

    if ([string]::IsNullOrWhiteSpace($Text)) {
        return $false
    }

    $lower = $Text.ToLowerInvariant()
    foreach ($needle in $Needles) {
        if (-not [string]::IsNullOrWhiteSpace($needle) -and $lower.Contains($needle.ToLowerInvariant())) {
            return $true
        }
    }
    return $false
}

function Get-Fb2CitationSourceKinds {
    param([object]$Data)

    $kinds = @()
    foreach ($source in @($Data.citation_sources | Where-Object { $_ })) {
        foreach ($field in @("kind", "source_kind", "source_type", "type")) {
            $property = $source.PSObject.Properties[$field]
            if ($null -ne $property -and -not [string]::IsNullOrWhiteSpace([string]$property.Value)) {
                $kinds += ([string]$property.Value).Trim()
                break
            }
        }
    }
    @($kinds | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Select-Object -Unique)
}

function Assert-Fb2ContextProjectionSection {
    param(
        [string]$ContextPack,
        [string]$SectionId,
        [string[]]$AcceptedMarkers,
        [string]$Scenario
    )

    $markers = @($SectionId) + @($AcceptedMarkers)
    Assert-True (Test-Fb2ProjectionTextContainsAny -Text $ContextPack -Needles $markers) "context projection section: $Scenario/$SectionId" ($markers -join "|")
}

function Assert-Fb2ContextPackProjection {
    param(
        [object]$Data,
        [string]$Scenario,
        [string[]]$ExpectedSourceKinds = @()
    )

    $contextPack = Get-Fb2ContextProjectionText -Data $Data
    Assert-True (-not [string]::IsNullOrWhiteSpace($contextPack)) "context projection body: $Scenario"
    Assert-True ($contextPack -match '<fb2_context_pack\b') "context projection wrapper open: $Scenario"
    Assert-True ($contextPack -match '</fb2_context_pack>') "context projection wrapper close: $Scenario"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$Data.context_audit_id)) "context projection audit id: $Scenario" "$($Data.context_audit_id)"
    Assert-True ($null -ne $Data.citation_sources) "context projection source registry: $Scenario"

    Assert-Fb2ContextProjectionSection $contextPack "usage_boundary" @("使用边界") $Scenario
    Assert-Fb2ContextProjectionSection $contextPack "match_facts" @("今日/近期比赛", "比赛与赔率") $Scenario
    Assert-Fb2ContextProjectionSection $contextPack "user_order_slice" @("当前用户订单", "用户订单/票据") $Scenario
    Assert-Fb2ContextProjectionSection $contextPack "platform_order_summary" @("平台/店铺订单摘要", "平台匿名摘要") $Scenario
    Assert-Fb2ContextProjectionSection $contextPack "group_opinion_slice" @("群讨论观点", "群友观点") $Scenario
    Assert-Fb2ContextProjectionSection $contextPack "retrieval_evidence" @("召回理由", "数据缺口") $Scenario
    Assert-Fb2ContextProjectionSection $contextPack "quality_feedback" @("质量回填", "质量反馈") $Scenario

    $sourceKinds = @(Get-Fb2CitationSourceKinds -Data $Data)
    foreach ($kind in $ExpectedSourceKinds) {
        Assert-ContainsValue $sourceKinds $kind "context projection source kind: $Scenario/$kind"
    }
}

function Assert-Fb2DomainScenarioMatrixContract {
    param([object[]]$ScenarioMatrix)

    $scenarios = @($ScenarioMatrix)
    $scenarioIds = @($scenarios | ForEach-Object { $_.id })
    Assert-True (($scenarioIds -contains "today_matches_analysis") -and ($scenarioIds -contains "my_ticket_analysis")) "domain projection scenario matrix: core scenarios"
    Assert-True (($scenarioIds -contains "platform_order_risk") -and ($scenarioIds -contains "group_opinion_summary")) "domain projection scenario matrix: aggregate scenarios"
    Assert-True (($scenarioIds -contains "selected_message_review") -and ($scenarioIds -contains "source_reference_audit")) "domain projection scenario matrix: audit scenarios"

    $ticketScenario = Find-EvalScenario $scenarios "my_ticket_analysis"
    Assert-ScenarioContains $ticketScenario "context_pack_sections" @("match_facts", "user_order_slice", "retrieval_evidence") "domain projection scenario my ticket sections"
    Assert-ScenarioContains $ticketScenario "required_request" @("external_user_id", "X-FB2-AI-CONTEXT-USER-ID") "domain projection scenario my ticket request"
    Assert-ScenarioContains $ticketScenario "required_source_kinds" @("user_order", "ticket", "match") "domain projection scenario my ticket source kinds"
    Assert-ScenarioContains $ticketScenario "acceptance_signals" @("only_current_user_orders", "matched_cited_sources") "domain projection scenario my ticket acceptance"

    $platformScenario = Find-EvalScenario $scenarios "platform_order_risk"
    Assert-ScenarioContains $platformScenario "required_request" @("include_platform_orders=true", "X-FB2-AI-CONTEXT-SCOPE=platform_order_summary") "domain projection scenario platform request"
    Assert-ScenarioContains $platformScenario "forbidden_outputs" @("single_user_order_detail", "user_identity_leak") "domain projection scenario platform forbidden"

    $opinionScenario = Find-EvalScenario $scenarios "group_opinion_summary"
    Assert-ScenarioContains $opinionScenario "context_pack_sections" @("group_opinion_slice", "quality_feedback") "domain projection scenario opinion sections"
    Assert-ScenarioContains $opinionScenario "feedback_routes" @("/api/main-project/context/opinion-adoption-summary") "domain projection scenario opinion feedback route"
    Assert-ScenarioContains $opinionScenario "acceptance_signals" @("opinion_adoption_count_non_synthetic", "memory_refs_present") "domain projection scenario opinion acceptance"

    $selectedScenario = Find-EvalScenario $scenarios "selected_message_review"
    Assert-ScenarioContains $selectedScenario "trigger_source_ids" @("selected_message_id") "domain projection scenario selected trigger source"
    Assert-ScenarioContains $selectedScenario "acceptance_signals" @("reply_rejects_guarantee_claims", "reply_has_risk_boundary") "domain projection scenario selected acceptance"
}

function New-Fb2ContextProjectionSelfTestData {
    [pscustomobject]@{
        context_audit_id = "audit-self-test"
        context_pack = @"
<fb2_context_pack>
## usage_boundary 使用边界
只用于比赛讨论和订单剖析参考，不承诺命中，不建议重注。
## match_facts 今日/近期比赛与赔率
- match_id=m1 odds_updated_at=2026-06-22T10:00:00+08:00
## user_order_slice 当前用户订单/票据
- order_id=o1 ticket_id=t1 visibility=current_user_only
## platform_order_summary 平台/店铺订单摘要
- platform_order_summary=summary-1 scope=anonymous_aggregate
## group_opinion_slice 群讨论观点
- message_id=gm1 opinion_memory_id=mem1 stance=neutral
## retrieval_evidence 召回理由和数据缺口
- context_audit_id=audit-self-test reason=topic_hint
## quality_feedback 质量回填口径
- main_request_id=req1 feedback_trigger=visible_mention cited_sources=[m1,o1]
</fb2_context_pack>
"@
        citation_sources = @(
            [pscustomobject]@{ kind = "match"; id = "m1"; label = "比赛 m1" },
            [pscustomobject]@{ kind = "odds"; id = "odds-m1"; label = "赔率 m1" },
            [pscustomobject]@{ kind = "user_order"; id = "o1"; label = "订单 o1" },
            [pscustomobject]@{ kind = "ticket"; id = "t1"; label = "票据 t1" },
            [pscustomobject]@{ kind = "group_message"; id = "gm1"; label = "群消息 gm1" },
            [pscustomobject]@{ kind = "opinion_memory"; id = "mem1"; label = "观点记忆 mem1" },
            [pscustomobject]@{ kind = "platform_order_summary"; id = "summary-1"; label = "平台匿名摘要" },
            [pscustomobject]@{ kind = "context_audit"; id = "audit-self-test"; label = "审计" }
        )
    }
}

function New-Fb2DomainScenarioMatrixSelfTestData {
    @(
        [pscustomobject]@{
            id = "today_matches_analysis"
            context_pack_sections = @("match_facts", "retrieval_evidence", "quality_feedback")
            primary_tools = @("match_analysis_brief")
            required_source_kinds = @("match", "odds", "context_audit")
            required_request = @("group_id", "topic_hint")
            acceptance_signals = @("reply_has_data_facts", "matched_cited_sources")
            forbidden_outputs = @("fabricated_odds")
        },
        [pscustomobject]@{
            id = "my_ticket_analysis"
            context_pack_sections = @("match_facts", "user_order_slice", "retrieval_evidence")
            required_request = @("external_user_id", "X-FB2-AI-CONTEXT-USER-ID")
            required_source_kinds = @("user_order", "ticket", "match")
            acceptance_signals = @("only_current_user_orders", "matched_cited_sources")
        },
        [pscustomobject]@{
            id = "platform_order_risk"
            required_request = @("include_platform_orders=true", "X-FB2-AI-CONTEXT-SCOPE=platform_order_summary")
            forbidden_outputs = @("single_user_order_detail", "user_identity_leak")
        },
        [pscustomobject]@{
            id = "group_opinion_summary"
            context_pack_sections = @("group_opinion_slice", "quality_feedback")
            feedback_routes = @("/api/main-project/context/opinion-adoption-summary")
            acceptance_signals = @("opinion_adoption_count_non_synthetic", "memory_refs_present")
        },
        [pscustomobject]@{
            id = "selected_message_review"
            trigger_source_ids = @("selected_message_id")
            acceptance_signals = @("reply_rejects_guarantee_claims", "reply_has_risk_boundary")
        },
        [pscustomobject]@{
            id = "source_reference_audit"
            context_pack_sections = @("retrieval_evidence", "quality_feedback")
            feedback_routes = @("/api/main-project/context/feedbacks")
        }
    )
}

function Invoke-ContextProjectionSelfTestCase {
    param(
        [string]$Name,
        [object]$Data,
        [string[]]$ExpectedSourceKinds,
        [bool]$ShouldPass
    )

    $before = $script:Failed
    Assert-Fb2ContextPackProjection -Data $Data -Scenario $Name -ExpectedSourceKinds $ExpectedSourceKinds
    $caseFailures = $script:Failed - $before
    $script:Failed = $before
    $passedExpectation = if ($ShouldPass) { $caseFailures -eq 0 } else { $caseFailures -gt 0 }
    if ($passedExpectation) {
        Write-Output "OK`tself-test context projection $Name`tcase_failures=$caseFailures"
    } else {
        $script:SelfTestFailed += 1
        Write-Output "FAIL`tself-test context projection $Name`tcase_failures=$caseFailures shouldPass=$ShouldPass"
    }
}

function Invoke-DomainScenarioMatrixSelfTestCase {
    param(
        [string]$Name,
        [object[]]$ScenarioMatrix,
        [bool]$ShouldPass
    )

    $before = $script:Failed
    Assert-Fb2DomainScenarioMatrixContract -ScenarioMatrix $ScenarioMatrix
    $caseFailures = $script:Failed - $before
    $script:Failed = $before
    $passedExpectation = if ($ShouldPass) { $caseFailures -eq 0 } else { $caseFailures -gt 0 }
    if ($passedExpectation) {
        Write-Output "OK`tself-test domain scenario matrix $Name`tcase_failures=$caseFailures"
    } else {
        $script:SelfTestFailed += 1
        Write-Output "FAIL`tself-test domain scenario matrix $Name`tcase_failures=$caseFailures shouldPass=$ShouldPass"
    }
}

function Invoke-Fb2ContextProjectionSelfTests {
    $valid = New-Fb2ContextProjectionSelfTestData
    Invoke-ContextProjectionSelfTestCase "valid domain pack" $valid @("match", "odds", "user_order", "context_audit") $true

    $missingWrapper = Copy-SelfTestObject $valid
    $missingWrapper.context_pack = $missingWrapper.context_pack.Replace("<fb2_context_pack>", "").Replace("</fb2_context_pack>", "")
    Invoke-ContextProjectionSelfTestCase "rejects missing wrapper" $missingWrapper @("match") $false

    $missingSection = Copy-SelfTestObject $valid
    $missingSection.context_pack = $missingSection.context_pack.Replace("## retrieval_evidence 召回理由和数据缺口", "## other_section")
    Invoke-ContextProjectionSelfTestCase "rejects missing retrieval section" $missingSection @("match") $false

    $missingSourceKind = Copy-SelfTestObject $valid
    $missingSourceKind.citation_sources = @($missingSourceKind.citation_sources | Where-Object { $_.kind -ne "odds" })
    Invoke-ContextProjectionSelfTestCase "rejects missing odds source" $missingSourceKind @("match", "odds") $false

    $validMatrix = New-Fb2DomainScenarioMatrixSelfTestData
    Invoke-DomainScenarioMatrixSelfTestCase "valid matrix" $validMatrix $true

    $missingTicketPermission = @($validMatrix | ForEach-Object { Copy-SelfTestObject $_ })
    $ticketScenario = Find-EvalScenario $missingTicketPermission "my_ticket_analysis"
    $ticketScenario.required_request = @("external_user_id")
    Invoke-DomainScenarioMatrixSelfTestCase "rejects missing ticket user header" $missingTicketPermission $false

    $missingOpinionAdoption = @($validMatrix | ForEach-Object { Copy-SelfTestObject $_ })
    $opinionScenario = Find-EvalScenario $missingOpinionAdoption "group_opinion_summary"
    $opinionScenario.feedback_routes = @("/api/main-project/context/feedback")
    Invoke-DomainScenarioMatrixSelfTestCase "rejects missing opinion adoption route" $missingOpinionAdoption $false
}
