#requires -Version 7.0

function Test-Fb2GoalEvidencePresent {
    param([object]$Value)

    return -not [string]::IsNullOrWhiteSpace([string]$Value)
}

function Test-Fb2GoalEvidenceZero {
    param([object]$Value)

    $text = [string]$Value
    if ([string]::IsNullOrWhiteSpace($text)) {
        return $false
    }
    return $text -match "\b(value|count)=0\b|^0$"
}

function Get-Fb2GoalCompletionState {
    param(
        [bool]$DataSuccess,
        [bool]$FeedbackComplete,
        [bool]$DataDirectReadComplete,
        [bool]$ReadOnlyDirectReadComplete,
        [bool]$VisibleAnswerPolicyComplete,
        [bool]$ContextProjectionComplete,
        [bool]$VoiceEvidencePathPresent,
        [bool]$FullFinalAcceptanceComplete = $false,
        [object]$FinalEvidence
    )

    $groupChatDirectReadReady = ($DataDirectReadComplete -or $ReadOnlyDirectReadComplete)
    $visibleAiFeedbackReady = ($DataSuccess -and $FeedbackComplete -and $DataDirectReadComplete -and $VisibleAnswerPolicyComplete)
    $contextDataReady = ($DataSuccess -and $ContextProjectionComplete)

    $hasMyTicketOrders = Test-Fb2GoalEvidencePresent $FinalEvidence.scenario_my_ticket_orders
    $hasPlatformSummary = Test-Fb2GoalEvidencePresent $FinalEvidence.scenario_platform_order_summary
    $hasPermissionAudit = Test-Fb2GoalEvidencePresent $FinalEvidence.permission_total_blocks
    $hasQualityNoUnmatched = Test-Fb2GoalEvidenceZero $FinalEvidence.quality_unmatched_cited_sources
    $hasOpinionAdoption = Test-Fb2GoalEvidencePresent $FinalEvidence.quality_non_synthetic_adoption_count
    $permissionQualityReady = ($hasPermissionAudit -and $hasQualityNoUnmatched -and $hasOpinionAdoption)

    $readyItems = @()
    $missingItems = @()

    if ($contextDataReady) { $readyItems += "fb2_context_pack_projection" } else { $missingItems += "fb2_context_pack_projection" }
    if ($hasMyTicketOrders) { $readyItems += "current_user_orders" } else { $missingItems += "current_user_orders" }
    if ($hasPlatformSummary) { $readyItems += "platform_order_summary" } else { $missingItems += "platform_order_summary" }
    if ($groupChatDirectReadReady) { $readyItems += "direct_group_chat_read" } else { $missingItems += "direct_group_chat_read" }
    if ($visibleAiFeedbackReady) { $readyItems += "visible_ai_feedback" } else { $missingItems += "visible_ai_feedback" }
    if ($VisibleAnswerPolicyComplete) { $readyItems += "visible_answer_policy" } else { $missingItems += "visible_answer_policy" }
    if ($permissionQualityReady) { $readyItems += "permission_quality_feedback" } else { $missingItems += "permission_quality_feedback" }
    if ($VoiceEvidencePathPresent) { $readyItems += "voice_final_evidence_path_present" } else { $missingItems += "voice_final_evidence_path_present" }
    if ($FullFinalAcceptanceComplete) { $readyItems += "same_batch_full_final_acceptance" } else { $missingItems += "same_batch_full_final_acceptance" }

    $nonVoiceReady = ($contextDataReady -and $hasMyTicketOrders -and $hasPlatformSummary -and $groupChatDirectReadReady -and $visibleAiFeedbackReady -and $VisibleAnswerPolicyComplete -and $permissionQualityReady)
    $fullFinalReady = ($nonVoiceReady -and $VoiceEvidencePathPresent -and $FullFinalAcceptanceComplete)

    $stage = if ($fullFinalReady) {
        "full_final_acceptance_ready"
    } elseif ($nonVoiceReady -and $VoiceEvidencePathPresent) {
        "non_voice_data_chat_permission_quality_ready_waiting_full_final_acceptance"
    } elseif ($nonVoiceReady) {
        "non_voice_data_chat_permission_quality_ready_voice_deferred"
    } elseif ($groupChatDirectReadReady -or $contextDataReady) {
        "partial_non_voice_ready"
    } else {
        "needs_data_context_and_chat_evidence"
    }

    $nextMinimumAction = if ($fullFinalReady) {
        "full_final_ready_verify_current_release_state"
    } elseif ($nonVoiceReady -and $VoiceEvidencePathPresent) {
        "run_full_final_acceptance_with_current_voice_evidence"
    } elseif ($nonVoiceReady) {
        "resume_ASR_TTS_later_and_provide_final_ready_voice_device_evidence"
    } else {
        "refresh_DataOnlyAcceptance_PreflightOnly_with_FB2_AI_CENTER_TOKEN"
    }

    [ordered]@{
        schema = "fb2.main_project.goal_completion.v1"
        stage = $stage
        non_voice_ready = $nonVoiceReady
        full_final_ready = $fullFinalReady
        ready_items = $readyItems
        missing_items = $missingItems
        requirements = [ordered]@{
            fb2_context_pack_projection = $contextDataReady
            current_user_orders = $hasMyTicketOrders
            platform_order_summary = $hasPlatformSummary
            direct_group_chat_read = $groupChatDirectReadReady
            visible_ai_feedback = $visibleAiFeedbackReady
            visible_answer_policy = $VisibleAnswerPolicyComplete
            permission_quality_feedback = $permissionQualityReady
            voice_final_evidence_path_present = $VoiceEvidencePathPresent
            same_batch_full_final_acceptance = $FullFinalAcceptanceComplete
        }
        next_minimum_action = $nextMinimumAction
    }
}
