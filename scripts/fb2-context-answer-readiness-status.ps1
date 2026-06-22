#requires -Version 7.0

function Get-Fb2ContextAnswerScenarioSpec {
    @(
        [ordered]@{
            id = "today_matches_analysis"
            sample_scenario = "today_matches_context_pack"
            user_question = "今天比赛怎么看"
            required_source_kinds = @("match", "odds", "context_audit")
            required_answer_layers = @("match_facts", "odds_facts", "ai_inference", "risk_boundary")
            forbidden_outputs = @("guaranteed_win", "fabricated_odds")
        },
        [ordered]@{
            id = "my_ticket_analysis"
            sample_scenario = "my_ticket_context_pack"
            user_question = "帮我分析我的票"
            required_source_kinds = @("user_order", "ticket", "context_audit")
            required_answer_layers = @("match_facts", "current_user_orders", "ai_inference", "risk_boundary")
            forbidden_outputs = @("other_user_order_detail", "guaranteed_win")
        },
        [ordered]@{
            id = "platform_order_risk"
            sample_scenario = "platform_order_context_pack"
            user_question = "平台今天订单风险怎么样"
            required_source_kinds = @("platform_order_summary", "context_audit")
            required_answer_layers = @("platform_aggregate", "ai_inference", "risk_boundary")
            forbidden_outputs = @("single_user_order_detail", "user_identity_leak")
        },
        [ordered]@{
            id = "group_opinion_summary"
            sample_scenario = "group_opinion_context_pack"
            user_question = "群里大家怎么看这场"
            required_source_kinds = @("group_message", "opinion_memory", "context_audit")
            required_answer_layers = @("group_opinion", "match_facts", "ai_inference", "risk_boundary")
            forbidden_outputs = @("group_opinion_as_fact", "fabricated_group_view")
        }
    )
}

function Get-Fb2ContextAnswerReadinessProperty {
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

function Find-Fb2ContextAnswerSampleScenario {
    param(
        [object]$SampleSetState,
        [string]$ScenarioId
    )

    foreach ($scenario in @((Get-Fb2ContextAnswerReadinessProperty $SampleSetState "scenarios" @()))) {
        if ([string](Get-Fb2ContextAnswerReadinessProperty $scenario "scenario" "") -eq $ScenarioId) {
            return $scenario
        }
    }
    return $null
}

function Get-Fb2ContextAnswerReadinessState {
    param([object]$SampleSetState)

    $sampleSetComplete = [bool](Get-Fb2ContextAnswerReadinessProperty $SampleSetState "complete" $false)
    $scenarioResults = @()
    $missing = @()

    foreach ($spec in Get-Fb2ContextAnswerScenarioSpec) {
        $sampleScenarioId = [string]$spec["sample_scenario"]
        $sampleScenario = Find-Fb2ContextAnswerSampleScenario -SampleSetState $SampleSetState -ScenarioId $sampleScenarioId
        $sourceKinds = @((Get-Fb2ContextAnswerReadinessProperty $sampleScenario "source_kinds" @()) | ForEach-Object { [string]$_ })
        $missingSourceKinds = @()
        foreach ($requiredKind in @($spec["required_source_kinds"])) {
            if ($sourceKinds -notcontains $requiredKind) {
                $missingSourceKinds += $requiredKind
            }
        }
        $scenarioPassed = $sampleSetComplete `
            -and $null -ne $sampleScenario `
            -and [bool](Get-Fb2ContextAnswerReadinessProperty $sampleScenario "passed" $false) `
            -and $missingSourceKinds.Count -eq 0 `
            -and -not [string]::IsNullOrWhiteSpace([string](Get-Fb2ContextAnswerReadinessProperty $sampleScenario "context_audit_id" "")) `
            -and [int](Get-Fb2ContextAnswerReadinessProperty $sampleScenario "citation_source_count" 0) -gt 0

        if (-not $scenarioPassed) {
            $missing += [string]$spec["id"]
        }

        $scenarioResults += [ordered]@{
            id = [string]$spec["id"]
            sample_scenario = $sampleScenarioId
            user_question = [string]$spec["user_question"]
            complete = $scenarioPassed
            required_source_kinds = @($spec["required_source_kinds"])
            present_source_kinds = @($sourceKinds)
            missing_source_kinds = @($missingSourceKinds)
            required_answer_layers = @($spec["required_answer_layers"])
            forbidden_outputs = @($spec["forbidden_outputs"])
            context_audit_id = [string](Get-Fb2ContextAnswerReadinessProperty $sampleScenario "context_audit_id" "")
            citation_source_count = [int](Get-Fb2ContextAnswerReadinessProperty $sampleScenario "citation_source_count" 0)
            context_pack_sha256 = [string](Get-Fb2ContextAnswerReadinessProperty $sampleScenario "context_pack_sha256" "")
        }
    }

    [ordered]@{
        schema = "fb2.main_project.context_answer_readiness.v1"
        complete = ($sampleSetComplete -and $missing.Count -eq 0)
        sample_set_complete = $sampleSetComplete
        scenario_count = @($scenarioResults).Count
        passed_count = @($scenarioResults | Where-Object { [bool]$_["complete"] }).Count
        failed_count = @($scenarioResults | Where-Object { -not [bool]$_["complete"] }).Count
        scenarios = @($scenarioResults)
        missing = @($missing)
        note = "offline_source_coverage_only_requires_live_model_feedback_with_FB2_AI_CENTER_TOKEN_for_final_acceptance"
    }
}
