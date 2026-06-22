#requires -Version 7.0

function Get-Fb2UserScenarioAuditProperty {
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

function Test-Fb2UserScenarioAuditTruthy {
    param([object]$Value)

    if ($null -eq $Value) {
        return $false
    }
    if ($Value -is [bool]) {
        return [bool]$Value
    }
    return ([string]$Value) -match "^(true|True|1)$"
}

function Test-Fb2UserScenarioAuditEvidencePresent {
    param([object]$Value)

    return -not [string]::IsNullOrWhiteSpace([string]$Value)
}

function Test-Fb2UserScenarioAuditZeroValue {
    param([object]$Value)

    $text = [string]$Value
    if ([string]::IsNullOrWhiteSpace($text)) {
        return $false
    }
    return $text -match "\b(value|count)=0\b|^0$"
}

function Find-Fb2UserScenarioAnswerReadiness {
    param(
        [object]$AnswerReadinessState,
        [string]$ScenarioId
    )

    foreach ($scenario in @((Get-Fb2UserScenarioAuditProperty $AnswerReadinessState "scenarios" @()))) {
        if ([string](Get-Fb2UserScenarioAuditProperty $scenario "id" "") -eq $ScenarioId) {
            return $scenario
        }
    }
    return $null
}

function New-Fb2UserScenarioAuditItem {
    param(
        [string]$Id,
        [string]$UserQuestion,
        [string]$EvidenceMode,
        [string[]]$RequiredSourceKinds,
        [string[]]$RequiredAnswerLayers,
        [string[]]$ForbiddenOutputs,
        [bool]$Complete,
        [string[]]$Missing,
        [object]$Evidence
    )

    [ordered]@{
        id = $Id
        user_question = $UserQuestion
        evidence_mode = $EvidenceMode
        complete = $Complete
        required_source_kinds = @($RequiredSourceKinds)
        required_answer_layers = @($RequiredAnswerLayers)
        forbidden_outputs = @($ForbiddenOutputs)
        missing = @($Missing)
        evidence = $Evidence
    }
}

function Get-Fb2UserScenarioAuditState {
    param(
        [object]$LatestData,
        [object]$LatestReadOnly,
        [object]$FinalEvidence,
        [object]$FeedbackCoverage,
        [object]$DataDirectReadState,
        [object]$ContextProjectionState,
        [object]$AnswerReadinessState
    )

    $scenarios = @()
    $missingScenarioIds = @()
    $answerScenarioIds = @(
        "today_matches_analysis",
        "my_ticket_analysis",
        "platform_order_risk",
        "group_opinion_summary"
    )

    foreach ($scenarioId in $answerScenarioIds) {
        $answerScenario = Find-Fb2UserScenarioAnswerReadiness -AnswerReadinessState $AnswerReadinessState -ScenarioId $scenarioId
        $complete = Test-Fb2UserScenarioAuditTruthy (Get-Fb2UserScenarioAuditProperty $answerScenario "complete")
        $missing = @((Get-Fb2UserScenarioAuditProperty $answerScenario "missing_source_kinds" @()))
        if (-not $complete) {
            $missingScenarioIds += $scenarioId
            if ($missing.Count -eq 0) {
                $missing += "answer_readiness"
            }
        }
        $scenarios += New-Fb2UserScenarioAuditItem `
            -Id $scenarioId `
            -UserQuestion ([string](Get-Fb2UserScenarioAuditProperty $answerScenario "user_question" "")) `
            -EvidenceMode "offline_context_pack_sample_source_coverage" `
            -RequiredSourceKinds @((Get-Fb2UserScenarioAuditProperty $answerScenario "required_source_kinds" @())) `
            -RequiredAnswerLayers @((Get-Fb2UserScenarioAuditProperty $answerScenario "required_answer_layers" @())) `
            -ForbiddenOutputs @((Get-Fb2UserScenarioAuditProperty $answerScenario "forbidden_outputs" @())) `
            -Complete $complete `
            -Missing @($missing) `
            -Evidence ([ordered]@{
                context_audit_id = [string](Get-Fb2UserScenarioAuditProperty $answerScenario "context_audit_id" "")
                citation_source_count = [int](Get-Fb2UserScenarioAuditProperty $answerScenario "citation_source_count" 0)
                context_pack_sha256 = [string](Get-Fb2UserScenarioAuditProperty $answerScenario "context_pack_sha256" "")
            })
    }

    $visibleEvidence = Get-Fb2UserScenarioAuditProperty $LatestData "visible_direct_read_evidence"
    $selectedSeed = [string](Get-Fb2UserScenarioAuditProperty $visibleEvidence "selected_message_seed" "")
    $selectedReply = [string](Get-Fb2UserScenarioAuditProperty $visibleEvidence "selected_message_reply" "")
    $selectedComplete = (
        Test-Fb2UserScenarioAuditEvidencePresent $selectedSeed `
        -and Test-Fb2UserScenarioAuditEvidencePresent $selectedReply `
        -and $selectedSeed -match "text_sha256=" `
        -and $selectedReply -match "text_sha256="
    )
    $selectedMissing = @()
    if (-not $selectedComplete) {
        $missingScenarioIds += "selected_message_review"
        if (-not (Test-Fb2UserScenarioAuditEvidencePresent $selectedSeed)) { $selectedMissing += "selected_message_seed_direct_read" }
        if (-not (Test-Fb2UserScenarioAuditEvidencePresent $selectedReply)) { $selectedMissing += "selected_message_reply_direct_read" }
        if ($selectedSeed -and $selectedSeed -notmatch "text_sha256=") { $selectedMissing += "selected_message_seed_text_hash" }
        if ($selectedReply -and $selectedReply -notmatch "text_sha256=") { $selectedMissing += "selected_message_reply_text_hash" }
    }
    $scenarios += New-Fb2UserScenarioAuditItem `
        -Id "selected_message_review" `
        -UserQuestion "这条消息说得对吗" `
        -EvidenceMode "visible_group_direct_read_evidence" `
        -RequiredSourceKinds @("selected_message", "match", "context_audit") `
        -RequiredAnswerLayers @("selected_message_fact", "match_facts", "ai_inference", "risk_boundary") `
        -ForbiddenOutputs @("selected_message_as_external_fact", "guaranteed_win") `
        -Complete $selectedComplete `
        -Missing @($selectedMissing) `
        -Evidence ([ordered]@{
            selected_message_seed = $selectedSeed
            selected_message_reply = $selectedReply
        })

    $summaryPost = [string](Get-Fb2UserScenarioAuditProperty $visibleEvidence "summary_post" "")
    $summaryReadyForMode = Test-Fb2UserScenarioAuditTruthy (Get-Fb2UserScenarioAuditProperty $LatestData "summary_post_ready_for_mode")
    $feedbackComplete = Test-Fb2UserScenarioAuditTruthy (Get-Fb2UserScenarioAuditProperty $FeedbackCoverage "complete")
    $summaryComplete = (
        Test-Fb2UserScenarioAuditEvidencePresent $summaryPost `
        -and $summaryPost -match "text_sha256=" `
        -and $summaryReadyForMode `
        -and $feedbackComplete
    )
    $summaryMissing = @()
    if (-not $summaryComplete) {
        $missingScenarioIds += "group_discussion_summary_post"
        if (-not (Test-Fb2UserScenarioAuditEvidencePresent $summaryPost)) { $summaryMissing += "summary_post_direct_read" }
        if ($summaryPost -and $summaryPost -notmatch "text_sha256=") { $summaryMissing += "summary_post_text_hash" }
        if (-not $summaryReadyForMode) { $summaryMissing += "summary_post_ready_for_mode" }
        if (-not $feedbackComplete) { $summaryMissing += "summary_post_feedback_coverage" }
    }
    $scenarios += New-Fb2UserScenarioAuditItem `
        -Id "group_discussion_summary_post" `
        -UserQuestion "总结今天群聊讨论" `
        -EvidenceMode "visible_summary_post_direct_read_and_feedback" `
        -RequiredSourceKinds @("group_message", "opinion_memory", "context_audit") `
        -RequiredAnswerLayers @("group_opinion", "match_facts", "ai_inference", "risk_boundary") `
        -ForbiddenOutputs @("fabricated_group_view", "guaranteed_win") `
        -Complete $summaryComplete `
        -Missing @($summaryMissing) `
        -Evidence ([ordered]@{
            summary_post = $summaryPost
            summary_post_ready_for_mode = $summaryReadyForMode
            feedback_complete = $feedbackComplete
        })

    $contextProjectionComplete = Test-Fb2UserScenarioAuditTruthy (Get-Fb2UserScenarioAuditProperty $ContextProjectionState "complete")
    $qualityUnmatchedZero = Test-Fb2UserScenarioAuditZeroValue (Get-Fb2UserScenarioAuditProperty $FinalEvidence "quality_unmatched_cited_sources")
    $sourceAuditComplete = ($contextProjectionComplete -and $qualityUnmatchedZero)
    $sourceAuditMissing = @()
    if (-not $sourceAuditComplete) {
        $missingScenarioIds += "source_reference_audit"
        if (-not $contextProjectionComplete) { $sourceAuditMissing += "context_projection_log" }
        if (-not $qualityUnmatchedZero) { $sourceAuditMissing += "quality_unmatched_cited_sources_zero" }
    }
    $scenarios += New-Fb2UserScenarioAuditItem `
        -Id "source_reference_audit" `
        -UserQuestion "你刚才依据了哪些比赛、订单和群消息" `
        -EvidenceMode "context_projection_and_quality_summary" `
        -RequiredSourceKinds @("context_audit", "match", "group_message") `
        -RequiredAnswerLayers @("source_registry", "data_fact_boundary", "quality_feedback") `
        -ForbiddenOutputs @("uncited_source", "fabricated_source") `
        -Complete $sourceAuditComplete `
        -Missing @($sourceAuditMissing) `
        -Evidence ([ordered]@{
            context_projection_complete = $contextProjectionComplete
            quality_unmatched_cited_sources = [string](Get-Fb2UserScenarioAuditProperty $FinalEvidence "quality_unmatched_cited_sources" "")
        })

    $completeCount = @($scenarios | Where-Object { [bool]$_["complete"] }).Count

    [ordered]@{
        schema = "fb2.main_project.user_scenario_audit.v1"
        context_format = "xml_wrapped_markdown_context_pack_with_json_metadata"
        mcp_status = "not_first_phase_use_rest_context_pack_and_tool_manifest_first"
        scenario_count = @($scenarios).Count
        complete_count = $completeCount
        failed_count = @($scenarios).Count - $completeCount
        complete = ($completeCount -eq @($scenarios).Count)
        scenarios = @($scenarios)
        missing = @($missingScenarioIds | Select-Object -Unique)
        note = "audits_real_user_questions_and_chat_entrypoints_without_storing_order_or_message_bodies"
    }
}
