#requires -Version 7.0

function Get-Fb2GoalGapAuditProperty {
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

function Test-Fb2GoalGapAuditTruthy {
    param([object]$Value)

    if ($null -eq $Value) {
        return $false
    }
    if ($Value -is [bool]) {
        return [bool]$Value
    }
    return ([string]$Value) -match "^(true|True|1)$"
}

function ConvertTo-Fb2GoalGapAuditText {
    param([object]$Value)

    if ($null -eq $Value) {
        return ""
    }
    return [string]$Value
}

function Get-Fb2GoalGapAuditState {
    param(
        [object]$LatestData,
        [object]$LatestReadOnly,
        [object]$FeedbackCoverage,
        [object]$DataDirectReadState,
        [object]$ContextProjectionState,
        [object]$SampleRequestState,
        [object]$SampleSetState,
        [object]$AnswerReadinessState,
        [object]$UserScenarioAudit,
        [object]$GoalCompletion,
        [string]$LatestDataPath,
        [string]$LatestReadOnlyPath,
        [string]$LatestAiCenterLogPath,
        [bool]$TokenPresent,
        [bool]$VoiceEvidencePathPresent
    )

    $completed = @()
    $missing = @()
    $deferredByUser = @()

    $latestDataSuccess = Test-Fb2GoalGapAuditTruthy (Get-Fb2GoalGapAuditProperty $LatestData "success")
    $feedbackComplete = Test-Fb2GoalGapAuditTruthy (Get-Fb2GoalGapAuditProperty $FeedbackCoverage "complete")
    $dataDirectReadComplete = Test-Fb2GoalGapAuditTruthy (Get-Fb2GoalGapAuditProperty $DataDirectReadState "complete")
    $readOnlyDirectReadComplete = Test-Fb2GoalGapAuditTruthy (Get-Fb2GoalGapAuditProperty $LatestReadOnly "direct_read_complete")
    $readOnlyWrites = Test-Fb2GoalGapAuditTruthy (Get-Fb2GoalGapAuditProperty $LatestReadOnly "writes")
    $readOnlyDirectReadComplete = ($readOnlyDirectReadComplete -and -not $readOnlyWrites)
    $directGroupChatReadComplete = ($dataDirectReadComplete -or $readOnlyDirectReadComplete)
    $contextProjectionComplete = Test-Fb2GoalGapAuditTruthy (Get-Fb2GoalGapAuditProperty $ContextProjectionState "complete")
    $sampleRequestComplete = Test-Fb2GoalGapAuditTruthy (Get-Fb2GoalGapAuditProperty $SampleRequestState "complete")
    $sampleSetComplete = Test-Fb2GoalGapAuditTruthy (Get-Fb2GoalGapAuditProperty $SampleSetState "complete")
    $answerReadinessComplete = Test-Fb2GoalGapAuditTruthy (Get-Fb2GoalGapAuditProperty $AnswerReadinessState "complete")
    $userScenarioAuditComplete = Test-Fb2GoalGapAuditTruthy (Get-Fb2GoalGapAuditProperty $UserScenarioAudit "complete")
    $nonVoiceReady = Test-Fb2GoalGapAuditTruthy (Get-Fb2GoalGapAuditProperty $GoalCompletion "non_voice_ready")
    $fullFinalReady = Test-Fb2GoalGapAuditTruthy (Get-Fb2GoalGapAuditProperty $GoalCompletion "full_final_ready")

    if ($directGroupChatReadComplete) { $completed += "direct_group_chat_read" } else { $missing += "direct_group_chat_read" }
    if ($contextProjectionComplete) { $completed += "context_pack_projection" } else { $missing += "context_pack_projection" }
    if ($sampleRequestComplete) { $completed += "context_pack_sample_request_ready" } else { $missing += "context_pack_sample_request" }
    if ($sampleSetComplete) { $completed += "context_pack_sample_set_validated" } else { $missing += "context_pack_sample_set_validation" }
    if ($answerReadinessComplete) { $completed += "context_answer_readiness_validated" } else { $missing += "context_answer_readiness" }
    if ($userScenarioAuditComplete) { $completed += "user_scenario_audit_validated" } else { $missing += "user_scenario_audit" }
    if ($latestDataSuccess -and $feedbackComplete -and $dataDirectReadComplete) {
        $completed += "live_data_only_visible_chat_feedback_historical_ready"
    } else {
        $missing += "live_data_only_visible_chat_feedback_refresh"
    }
    if ($nonVoiceReady) { $completed += "non_voice_historical_evidence_ready" } else { $missing += "non_voice_historical_evidence" }

    if (-not $TokenPresent) {
        $missing += "FB2_AI_CENTER_TOKEN_live_permission_quality_refresh"
    }
    if (-not $VoiceEvidencePathPresent) {
        $missing += "voice_final_evidence"
        $deferredByUser += "ASR_TTS_final_evidence"
    }
    if (-not $fullFinalReady) {
        $missing += "full_final_acceptance_same_batch_voice_and_visible_chat"
    }

    $nextSmallestAction = if (-not $TokenPresent -and $answerReadinessComplete) {
        "set_FB2_AI_CENTER_TOKEN_then_run_DataOnlyAcceptance_PreflightOnly"
    } elseif (-not $TokenPresent -and $sampleSetComplete) {
        "set_FB2_AI_CENTER_TOKEN_to_refresh_live_permission_quality_after_offline_samples"
    } elseif (-not $TokenPresent -and $sampleRequestComplete) {
        "send_context_pack_sample_request_to_fb2_or_set_FB2_AI_CENTER_TOKEN"
    } elseif (-not $TokenPresent) {
        "generate_context_pack_sample_request_or_set_FB2_AI_CENTER_TOKEN"
    } elseif (-not $latestDataSuccess -or -not $dataDirectReadComplete -or -not $feedbackComplete) {
        "run_DataOnlyAcceptance_PreflightOnly_to_refresh_live_context_permission_quality_summary"
    } elseif (-not $VoiceEvidencePathPresent) {
        "keep_ASR_TTS_deferred_until_final_ready_voice_device_evidence_is_available"
    } elseif (-not $fullFinalReady) {
        "run_full_final_acceptance_with_current_voice_evidence"
    } else {
        "full_final_ready_verify_current_release_state"
    }

    [ordered]@{
        schema = "fb2.main_project.goal_gap_audit.v1"
        completed = @($completed | Select-Object -Unique)
        missing = @($missing | Select-Object -Unique)
        blocked_by_external_secret = (-not $TokenPresent)
        deferred_by_user = @($deferredByUser | Select-Object -Unique)
        next_smallest_action = $nextSmallestAction
        direct_read_policy = [ordered]@{
            required = $true
            screenshots_accepted = $false
            write_required_for_status = $false
            group_messages_api = "/api/me/groups/{group_id}/messages"
            summary_posts_api = "/api/me/groups/{group_id}/summary-posts/{post_id}"
            required_fingerprint_fields = @("message_or_post_id", "text_len", "text_sha256")
        }
        current_flags = [ordered]@{
            token_present = $TokenPresent
            voice_evidence_path_present = $VoiceEvidencePathPresent
            data_only_success = $latestDataSuccess
            feedback_complete = $feedbackComplete
            data_direct_read_complete = $dataDirectReadComplete
            read_only_direct_read_complete = $readOnlyDirectReadComplete
            direct_group_chat_read_complete = $directGroupChatReadComplete
            context_projection_complete = $contextProjectionComplete
            sample_request_complete = $sampleRequestComplete
            sample_set_complete = $sampleSetComplete
            answer_readiness_complete = $answerReadinessComplete
            non_voice_ready = $nonVoiceReady
            full_final_ready = $fullFinalReady
        }
        evidence_refs = [ordered]@{
            data_only_summary_path = $LatestDataPath
            read_only_direct_read_summary_path = $LatestReadOnlyPath
            ai_center_log_path = $LatestAiCenterLogPath
            read_only_group_id = ConvertTo-Fb2GoalGapAuditText (Get-Fb2GoalGapAuditProperty $LatestReadOnly "group_id")
            read_only_sample_message_id = ConvertTo-Fb2GoalGapAuditText (Get-Fb2GoalGapAuditProperty $LatestReadOnly "sample_message_id")
            read_only_sample_text_sha256 = ConvertTo-Fb2GoalGapAuditText (Get-Fb2GoalGapAuditProperty $LatestReadOnly "sample_text_sha256")
            sample_set_path = ConvertTo-Fb2GoalGapAuditText (Get-Fb2GoalGapAuditProperty $SampleSetState "path")
            sample_set_passed_count = [int](Get-Fb2GoalGapAuditProperty $SampleSetState "passed_count" 0)
            sample_set_source_kinds = @((Get-Fb2GoalGapAuditProperty $SampleSetState "source_kinds" @()))
            answer_readiness_passed_count = [int](Get-Fb2GoalGapAuditProperty $AnswerReadinessState "passed_count" 0)
            user_scenario_complete_count = [int](Get-Fb2GoalGapAuditProperty $UserScenarioAudit "complete_count" 0)
            goal_completion_stage = ConvertTo-Fb2GoalGapAuditText (Get-Fb2GoalGapAuditProperty $GoalCompletion "stage")
        }
    }
}
