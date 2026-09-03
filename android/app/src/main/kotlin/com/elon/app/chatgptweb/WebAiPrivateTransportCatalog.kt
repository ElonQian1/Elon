package com.elon.app.chatgptweb

import com.elon.app.BuildConfig
import org.json.JSONArray
import org.json.JSONObject

/** Runtime inventory for private transports and intentionally unsupported shortcuts. */
internal object WebAiPrivateTransportCatalog {
    fun describe(): JSONArray = JSONArray().apply {
        entries().forEach { put(it.toJson()) }
    }

    private fun entries(): List<Entry> = listOf(
        Entry(
            id = "android_chatgpt_private_conversation_project_directory_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "passive_device_verified_targeted_get_contract_passed",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "passive_observer_and_targeted_same_origin_project_get",
            healthPolicy = "validated_project_id_singleflight_and_bounded_success_only_payload",
            fallback = "official_dom_directory",
        ),
        Entry(
            id = "android_chatgpt_private_conversation_prefetch_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "device_verified",
            productionDefault = true,
            runtimeEnabled = BuildConfig.CHATGPT_PRIVATE_CONVERSATION_PREFETCH_ENABLED,
            requestMode = "authenticated_same_origin_get",
            healthPolicy = "bounded_timeout_cooldown_and_circuit_breaker",
            fallback = "official_webview_navigation",
        ),
        Entry(
            id = "android_chatgpt_conversation_navigation_receipt_reconciliation_v1",
            provider = "chatgpt",
            status = "implemented_device_pending",
            verification = "targeted_exact_identity_reconciliation_tests_passed",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "exact_target_snapshot_navigation_receipt_reconciliation",
            healthPolicy = "request_id_target_identity_timeout_and_supersession_boundaries",
            fallback = "official_webview_navigation_without_write_replay",
        ),
        Entry(
            id = "android_chatgpt_private_send_dispatch_observer_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "device_structural_verified",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "passive_official_request_observer",
            healthPolicy = "acknowledge_dispatch_only",
            fallback = "official_dom_send_confirmation",
        ),
        Entry(
            id = "android_chatgpt_private_stream_observer_v1",
            provider = "chatgpt",
            status = "completed",
            verification =
                "device_verified_completion_v1_1_1302_and_structural_sparse_watchdog_v1_1_1310",
            productionDefault = true,
            runtimeEnabled = BuildConfig.CHATGPT_PRIVATE_STREAM_OBSERVER_ENABLED,
            requestMode = "passive_official_response_clone",
            healthPolicy = "private_event_driven_stream_with_sparse_dom_watchdog",
            fallback = "official_dom_stream_snapshot",
        ),
        Entry(
            id = "android_chatgpt_private_stream_completion_settlement_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "device_verified_v1_1_1302",
            productionDefault = true,
            runtimeEnabled = BuildConfig.CHATGPT_PRIVATE_STREAM_OBSERVER_ENABLED,
            requestMode = "passive_private_completion_reconciliation",
            healthPolicy = "official_stop_control_wins_otherwise_release_stale_native_stream",
            fallback = "official_dom_stream_snapshot",
        ),
        Entry(
            id = "android_chatgpt_realtime_voice_private_transcript_refresh_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "targeted_tests_passed_device_pending",
            productionDefault = true,
            runtimeEnabled = BuildConfig.CHATGPT_PRIVATE_CONVERSATION_PREFETCH_ENABLED,
            requestMode = "event_first_active_same_origin_get_current_conversation_reconciliation",
            healthPolicy =
                "single_flight_timeout_cooldown_circuit_breaker_and_sparse_dom_watchdog",
            fallback = "retained_native_transcript_and_official_dom_snapshot",
        ),
        Entry(
            id = "android_google_web_private_conversation_directory_v1",
            provider = "google_web_ai",
            status = "completed",
            verification = "device_verified",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "passive_official_response_observer",
            healthPolicy = "bounded_payload_with_durable_official_freshness",
            fallback = "local_directory_cache_and_official_page",
        ),
        Entry(
            id = "android_google_web_conversation_snapshot_cache_v1",
            provider = "google_web_ai",
            status = "completed",
            verification = "device_verified_cache_first_then_official_navigation",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "passive_official_snapshot_cache",
            healthPolicy = "validated_conversation_url_30d_128_items_24mib",
            fallback = "official_webview_navigation",
        ),
        Entry(
            id = "android_google_web_private_reply_observer_v1",
            provider = "google_web_ai",
            status = "completed",
            verification = "device_verified_stream_to_completion_v1_1_1303",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "passive_completion_signal",
            healthPolicy = "fast_initial_probe_then_sparse_stream_watchdog_and_dom_reconciliation",
            fallback = "official_dom_reply_snapshot",
        ),
        Entry(
            id = "android_web_ai_background_navigation_continuity_v1",
            provider = "chatgpt_and_google_web_ai",
            status = "completed",
            verification = "device_structural_verified",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "preserve_inflight_official_navigation",
            healthPolicy = "reattach_adapter_then_bounded_watchdog_and_reload",
            fallback = "official_webview_bounded_recovery",
        ),
        Entry(
            id = "android_web_ai_unified_send_coordinator_v1",
            provider = "chatgpt_and_google_web_ai",
            status = "completed",
            verification = "all_send_entry_points_targeted_tests_passed_device_pending",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "single_owner_command_ledger_with_official_page_transport",
            healthPolicy =
                "stable_request_id_acceptance_unknown_completion_and_page_reconciliation",
            fallback = "official_page_reconciliation_without_automatic_write_replay",
        ),
        Entry(
            id = "android_chatgpt_same_origin_text_transaction_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "device_verified_v1_1_1365_adapter_206",
            productionDefault = true,
            runtimeEnabled = BuildConfig.CHATGPT_PRIVATE_TEXT_TRANSACTIONS_ENABLED,
            requestMode =
                "versioned_single_flight_same_origin_text_post_when_reusable_else_official_page",
            healthPolicy =
                "dynamic_proof_gate_15s_timeout_two_failure_45s_cooldown_and_read_only_reconciliation",
            fallback = "immediate_official_page_transaction_without_write_replay",
            directPostEnabled = BuildConfig.CHATGPT_PRIVATE_TEXT_TRANSACTIONS_ENABLED,
        ),
        Entry(
            id = "android_chatgpt_interaction_preset_cache_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "device_verified_v1_1_1367",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode =
                "built_in_preset_persistent_cache_and_single_intent_official_reconciliation",
            healthPolicy =
                "stale_while_refresh_live_semantic_id_bounded_poll_and_exactly_once_dispatch",
            fallback = "current_official_control_and_webview_navigation",
        ),
        Entry(
            id = "android_chatgpt_attachment_transport_reconciliation_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "device_verified_v1_1_1373_adapter_207",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "passive_same_origin_upload_completion_observer",
            healthPolicy =
                "arm_on_native_picker_redacted_sequence_dedupe_and_stable_snapshot_gate",
            fallback = "official_dom_attachment_snapshot_and_bounded_timeout",
        ),
        Entry(
            id = "android_chatgpt_native_image_asset_gallery_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "device_verified_v1_1_1375_adapter_208",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "opaque_same_origin_image_assets_and_cache_first_native_gallery",
            healthPolicy =
                "bounded_jpeg_cache_six_hour_freshness_missing_handle_sync_and_timeout",
            fallback = "official_images_page",
        ),
        Entry(
            id = "android_chatgpt_native_image_generation_status_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "targeted_lifecycle_tests_passed_device_ui_pending",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode =
                "selected_official_tool_stream_state_and_opaque_image_asset_projection",
            healthPolicy =
                "official_stream_authority_bounded_preview_queue_retry_and_terminal_hide",
            fallback = "official_composer_and_images_page",
        ),
        Entry(
            id = "android_chatgpt_private_rich_content_native_view_v1",
            provider = "chatgpt",
            status = "completed",
            verification =
                "device_structural_verified_v1_1_1379_parser_contract_and_native_finance_chart_preview",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "passive_private_response_sanitized_rich_card_projection",
            healthPolicy =
                "versioned_schema_bounded_finance_chart_ast_unknown_drop_and_native_detail",
            fallback = "official_webview_rich_content",
        ),
        Entry(
            id = "android_chatgpt_native_dictation_v1",
            provider = "chatgpt",
            status = "implemented_device_pending",
            verification = "targeted_shared_bridge_composer_tests_pending",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "existing_work_mode_agent_voice_bridge_to_current_draft",
            healthPolicy =
                "existing_engine_ownership_and_bounded_unavailable_cooldown",
            fallback = "none_explicit_long_press_selection_only",
        ),
        Entry(
            id = "android_chatgpt_private_dictation_transport_v1",
            provider = "chatgpt",
            status = "completed",
            verification =
                "device_same_origin_synthetic_audio_endpoint_and_targeted_integration_tests_passed",
            productionDefault = true,
            runtimeEnabled = BuildConfig.CHATGPT_PRIVATE_DICTATION_ENABLED,
            requestMode =
                "same_origin_page_local_media_recorder_buffered_transcription",
            healthPolicy =
                "pre_capture_auth_gate_capture_ownership_bounded_timeouts_and_session_circuit_breaker",
            fallback = "none_explicit_long_press_shared_work_mode_selection",
            directPostEnabled = BuildConfig.CHATGPT_PRIVATE_DICTATION_ENABLED,
        ),
        Entry(
            id = "android_chatgpt_native_response_read_aloud_v1",
            provider = "chatgpt",
            status = "implemented_device_pending",
            verification = "targeted_chunking_and_message_action_tests_passed",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "native_bounded_chunk_tts_for_current_response",
            healthPolicy = "single_active_message_interruptible_full_text_chunk_sequence",
            fallback = "official_message_actions_and_webview",
        ),
        Entry(
            id = "android_chatgpt_native_conversation_project_move_v1",
            provider = "chatgpt",
            status = "implemented_device_pending",
            verification = "direct_dom_activation_confirmation_and_scoped_refresh_tests_pending",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode =
                "cached_native_destination_then_exact_official_dom_activation_and_optional_confirmation",
            healthPolicy =
                "request_receipt_target_directory_reconciliation_and_no_ambiguous_replay",
            fallback = "official_conversation_project_menu",
        ),
        Entry(
            id = "android_chatgpt_native_conversation_management_v1",
            provider = "chatgpt",
            status = "completed",
            verification =
                "targeted_action_policy_and_adaptive_control_tests_passed_device_mutations_pending",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode =
                "context_bound_official_controls_with_native_adaptive_forms",
            healthPolicy =
                "conversation_identity_scope_stale_control_rejection_confirmation_and_no_write_replay",
            fallback = "official_conversation_options",
        ),
        Entry(
            id = "android_chatgpt_realtime_voice_background_overlay_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "device_handoff_verified_manual_overlay_actions_pending",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "official_webrtc_foreground_service_and_local_overlay",
            healthPolicy =
                "accepted_hangup_snapshot_events_with_stable_window_and_sparse_watchdog",
            fallback = "official_webview_voice_and_foreground_notification",
        ),
        Entry(
            id = "android_chatgpt_private_transport_research_v1",
            provider = "chatgpt",
            status = "research_only",
            verification = "controlled_device_shapes_verified",
            productionDefault = false,
            runtimeEnabled = BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED,
            requestMode = "redacted_passive_diagnostics",
            healthPolicy = "disabled_in_production",
            fallback = "none",
        ),
        Entry(
            id = "android_chatgpt_web_private_voice_bootstrap_research_v1",
            provider = "chatgpt",
            status = "research_completed",
            verification = "device_form_session_and_webrtc_sequence_verified",
            productionDefault = false,
            runtimeEnabled = BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED,
            requestMode = "redacted_page_local_webrtc_and_bootstrap_observer",
            healthPolicy = "in_memory_shapes_only_no_credentials_sdp_or_ice",
            fallback = "official_page_created_webrtc",
        ),
        Entry(
            id = "android_chatgpt_web_private_voice_native_relay_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "device_native_single_audio_and_data_channel_verified",
            productionDefault = true,
            runtimeEnabled = BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED,
            requestMode = "same_origin_in_memory_session_relay_and_native_webrtc",
            healthPolicy =
                "single_use_expiry_no_persistence_connect_timeout_takeover_lock_and_official_fallback",
            fallback = "official_page_created_webrtc",
        ),
        Entry(
            id = "android_chatgpt_realtime_voice_data_channel_transcript_v1",
            provider = "chatgpt",
            status = "completed",
            verification =
                "device_private_delta_shape_observed_native_peer_connected_targeted_tests_passed",
            productionDefault = true,
            runtimeEnabled = BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED,
            requestMode = "passive_native_webrtc_private_delta_transcript_events",
            healthPolicy =
                "bounded_utf8_delta_decoder_deduplicated_in_memory_stream",
            fallback = "private_conversation_refresh_and_official_dom_snapshot",
        ),
        Entry(
            id = "android_google_web_private_response_research_v1",
            provider = "google_web_ai",
            status = "research_only",
            verification = "controlled_device_shapes_verified",
            productionDefault = false,
            runtimeEnabled = BuildConfig.GOOGLE_WEB_PRIVATE_RESEARCH_ENABLED,
            requestMode = "redacted_passive_diagnostics",
            healthPolicy = "disabled_in_production",
            fallback = "none",
        ),
        Entry(
            id = "android_google_web_direct_send_decision_v1",
            provider = "google_web_ai",
            status = "audited_no_safe_gain",
            verification = "source_and_contract_verified",
            productionDefault = false,
            runtimeEnabled = false,
            requestMode = "official_page_action_only",
            healthPolicy = "not_applicable",
            fallback = "official_form_navigation_and_dom_confirmation",
        ),
        Entry(
            id = "android_google_web_conversation_prefetch_decision_v1",
            provider = "google_web_ai",
            status = "deferred_no_observed_endpoint",
            verification = "controlled_endpoint_inventory_verified",
            productionDefault = false,
            runtimeEnabled = false,
            requestMode = "no_direct_request",
            healthPolicy = "not_applicable",
            fallback = "local_snapshot_then_official_navigation",
        ),
        Entry(
            id = "android_chatgpt_realtime_voice_session_reuse_decision_v1",
            provider = "chatgpt",
            status = "audited_no_safe_gain",
            verification = "source_and_protocol_boundary_verified",
            productionDefault = false,
            runtimeEnabled = false,
            requestMode = "official_webrtc_session_per_start",
            healthPolicy = "cache_launch_hints_not_live_credentials",
            fallback = "cached_voice_control_then_official_webrtc_start",
        ),
    )

    private data class Entry(
        val id: String,
        val provider: String,
        val status: String,
        val verification: String,
        val productionDefault: Boolean,
        val runtimeEnabled: Boolean,
        val requestMode: String,
        val healthPolicy: String,
        val fallback: String,
        val directPostEnabled: Boolean = false,
    ) {
        fun toJson(): JSONObject = JSONObject()
            .put("capability_id", id)
            .put("provider", provider)
            .put("implementation_status", status)
            .put("verification_status", verification)
            .put("production_default", productionDefault)
            .put("runtime_enabled", runtimeEnabled)
            .put("request_mode", requestMode)
            .put("health_policy", healthPolicy)
            .put("fallback", fallback)
            .put("direct_post_enabled", directPostEnabled)
            .put("official_page_authoritative", true)
    }
}
