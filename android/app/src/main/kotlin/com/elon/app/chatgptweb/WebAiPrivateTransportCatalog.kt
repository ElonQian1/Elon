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
            verification = "device_verified",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "passive_official_response_observer",
            healthPolicy = "emit_success_only_bounded_payload",
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
            verification = "device_verified",
            productionDefault = true,
            runtimeEnabled = BuildConfig.CHATGPT_PRIVATE_STREAM_OBSERVER_ENABLED,
            requestMode = "passive_official_response_clone",
            healthPolicy = "bounded_stream_observer_with_dom_reconciliation",
            fallback = "official_dom_stream_snapshot",
        ),
        Entry(
            id = "android_chatgpt_realtime_voice_private_transcript_refresh_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "targeted_tests_passed_device_pending",
            productionDefault = true,
            runtimeEnabled = BuildConfig.CHATGPT_PRIVATE_CONVERSATION_PREFETCH_ENABLED,
            requestMode = "authenticated_same_origin_get_current_conversation",
            healthPolicy = "single_flight_timeout_cooldown_and_circuit_breaker",
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
            id = "android_google_web_private_reply_observer_v1",
            provider = "google_web_ai",
            status = "completed",
            verification = "device_verified",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "passive_completion_signal",
            healthPolicy = "bounded_probe_then_dom_reconciliation",
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
            id = "android_chatgpt_realtime_voice_background_overlay_v1",
            provider = "chatgpt",
            status = "completed",
            verification = "device_handoff_verified_manual_overlay_actions_pending",
            productionDefault = true,
            runtimeEnabled = true,
            requestMode = "official_webrtc_foreground_service_and_local_overlay",
            healthPolicy =
                "fresh_grant_or_current_official_voice_control_activation_and_accepted_hangup_only_reconciliation",
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
            .put("direct_post_enabled", false)
            .put("official_page_authoritative", true)
    }
}
