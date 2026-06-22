#requires -Version 7.0

function Get-Fb2CoordinationJsonProperty {
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

function Test-Fb2CoordinationTruthyJsonValue {
    param([object]$Value)

    if ($null -eq $Value) {
        return $false
    }
    if ($Value -is [bool]) {
        return [bool]$Value
    }
    return ([string]$Value) -match "^(true|True|1)$"
}

function ConvertTo-Fb2CoordinationText {
    param([object]$Value)

    if ($null -eq $Value) {
        return ""
    }
    return [string]$Value
}

function Get-Fb2CoordinationContextProjectionStatus {
    param([object]$ContextProjectionState)

    $today = Get-Fb2CoordinationJsonProperty $ContextProjectionState "today_matches_context_pack"
    $ticket = Get-Fb2CoordinationJsonProperty $ContextProjectionState "my_ticket_context_pack"
    $businessData = Get-Fb2CoordinationJsonProperty $ContextProjectionState "business_data_checks"

    [ordered]@{
        complete = Test-Fb2CoordinationTruthyJsonValue (Get-Fb2CoordinationJsonProperty $ContextProjectionState "complete")
        today_matches_context_pack_complete = Test-Fb2CoordinationTruthyJsonValue (Get-Fb2CoordinationJsonProperty $today "complete")
        my_ticket_context_pack_complete = Test-Fb2CoordinationTruthyJsonValue (Get-Fb2CoordinationJsonProperty $ticket "complete")
        group_opinion_summary_present = Test-Fb2CoordinationTruthyJsonValue (Get-Fb2CoordinationJsonProperty $businessData "group_opinion_summary")
        platform_order_summary_present = Test-Fb2CoordinationTruthyJsonValue (Get-Fb2CoordinationJsonProperty $businessData "platform_order_summary")
        unmatched_sources_zero = Test-Fb2CoordinationTruthyJsonValue (Get-Fb2CoordinationJsonProperty $businessData "quality_unmatched_sources_zero")
        non_synthetic_adoption_present = Test-Fb2CoordinationTruthyJsonValue (Get-Fb2CoordinationJsonProperty $businessData "non_synthetic_opinion_adoption")
    }
}

function Get-Fb2AiCenterCoordinationStatus {
    param(
        [object]$LatestData,
        [object]$LatestReadOnly,
        [object]$FeedbackCoverage,
        [object]$FinalEvidence,
        [object]$DataDirectReadState,
        [object]$ContextProjectionState,
        [object]$GoalCompletion,
        [string]$LatestDataPath,
        [string]$LatestReadOnlyPath,
        [string]$LatestAiCenterLogPath,
        [bool]$TokenPresent,
        [bool]$VoiceEvidencePathPresent
    )

    $visibleEvidence = Get-Fb2CoordinationJsonProperty $LatestData "visible_direct_read_evidence"
    $nonVoiceReady = Test-Fb2CoordinationTruthyJsonValue (Get-Fb2CoordinationJsonProperty $GoalCompletion "non_voice_ready")
    $fullFinalReady = Test-Fb2CoordinationTruthyJsonValue (Get-Fb2CoordinationJsonProperty $GoalCompletion "full_final_ready")
    $readOnlyComplete = Test-Fb2CoordinationTruthyJsonValue (Get-Fb2CoordinationJsonProperty $LatestReadOnly "direct_read_complete")
    $dataDirectReadComplete = Test-Fb2CoordinationTruthyJsonValue (Get-Fb2CoordinationJsonProperty $DataDirectReadState "complete")

    [ordered]@{
        schema = "fb2.main_project.coordination.v1"
        summary = if ($fullFinalReady) {
            "full_final_ready"
        } elseif ($nonVoiceReady) {
            "non_voice_ready_voice_deferred"
        } else {
            "needs_live_data_refresh"
        }
        owner_split = [ordered]@{
            main_project = @(
                "AI Center contract, prompt projection, tool planning, answer policy, billing policy",
                "group/chat smoke wrappers and direct-read evidence summaries",
                "chat voice SDK public contract, when ASR/TTS work resumes"
            )
            fb2_project = @(
                "live match, odds, user order, platform summary, group opinion, feedback, and quality endpoints",
                "APK/client integration of main-project chat and voice SDK",
                "final-ready device voice evidence, when ASR/TTS work resumes"
            )
            forbidden = @(
                "do_not_copy_fb2_business_data_into_main_project",
                "do_not_use_screenshots_as_group_chat_acceptance_evidence",
                "do_not_run_visible_group_writes_without_explicit_authorization"
            )
        }
        acceptance_scope = [ordered]@{
            non_voice_ready = $nonVoiceReady
            full_final_ready = $fullFinalReady
            asr_tts_status = if ($VoiceEvidencePathPresent) { "voice_evidence_path_configured_but_not_verified_here" } else { "deferred_by_user_or_missing" }
            token_present_for_refresh = $TokenPresent
        }
        current_evidence = [ordered]@{
            data_only_summary_path = $LatestDataPath
            read_only_direct_read_summary_path = $LatestReadOnlyPath
            ai_center_log_path = $LatestAiCenterLogPath
            external_user_id = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $LatestData "external_user_id")
            fb2_group_id = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $LatestData "group_id")
            visible_group_id = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $LatestData "visible_group_id" (Get-Fb2CoordinationJsonProperty $LatestReadOnly "group_id"))
            visible_mention_seed_id = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $LatestData "visible_mention_message_id")
            visible_mention_reply_id = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $LatestData "visible_mention_reply_id")
            selected_message_seed_id = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $LatestData "selected_message_seed_id")
            selected_message_reply_id = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $LatestData "selected_message_reply_id")
            summary_post_id = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $LatestData "summary_post_id")
            summary_post_status = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $LatestData "summary_post_status")
            feedback_complete = Test-Fb2CoordinationTruthyJsonValue (Get-Fb2CoordinationJsonProperty $FeedbackCoverage "complete")
            feedback_observed_count = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $FeedbackCoverage "observed_count")
            data_direct_read_complete = $dataDirectReadComplete
            data_direct_read_mode = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $DataDirectReadState "mode")
            read_only_direct_read_complete = $readOnlyComplete
            read_only_sample_message_id = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $LatestReadOnly "sample_message_id")
            read_only_sample_text_sha256 = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $LatestReadOnly "sample_text_sha256")
            baseline_messages = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $visibleEvidence "baseline_messages")
            visible_mention_reply = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $visibleEvidence "visible_mention_reply")
            selected_message_reply = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $visibleEvidence "selected_message_reply")
            summary_post = ConvertTo-Fb2CoordinationText (Get-Fb2CoordinationJsonProperty $visibleEvidence "summary_post")
        }
        context_projection = Get-Fb2CoordinationContextProjectionStatus $ContextProjectionState
        direct_read_policy = [ordered]@{
            group_messages_api = "/api/me/groups/{group_id}/messages"
            summary_posts_api = "/api/me/groups/{group_id}/summary-posts/{post_id}"
            screenshots_accepted = $false
            writes_group_messages_in_status = $false
            stores_message_body = $false
            required_fingerprint_fields = @("message_or_post_id", "text_len", "text_sha256")
        }
        safe_commands = [ordered]@{
            no_write_direct_read = 'pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-visible-chat.ps1 -ReadOnlyDirectRead -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD>'
            data_only_preflight = 'pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD> -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>'
            visible_regression_requires_authorization = 'pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -AllowVisibleMessages -Fb2Username 123qwe -Fb2Password <FB2_PASSWORD> -Fb2AiCenterToken <FB2_AI_CENTER_TOKEN>'
        }
        next_action_by_owner = [ordered]@{
            main_project = if ($nonVoiceReady) {
                "keep contracts/status wrappers current; do not reopen ASR/TTS until requested"
            } else {
                "refresh data-only preflight with service token and inspect failing contract evidence"
            }
            fb2_project = if ($nonVoiceReady) {
                "treat non-voice data/chat/feedback as passed; continue using direct APIs for regressions"
            } else {
                "provide current service token/live endpoint evidence and fix any failing context endpoint"
            }
            shared = if ($fullFinalReady) {
                "run full final acceptance"
            } else {
                "ASR/TTS final evidence remains outside current non-voice scope"
            }
        }
        missing_for_full_final = @((Get-Fb2CoordinationJsonProperty $GoalCompletion "missing_items" @()))
    }
}
