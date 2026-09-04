package com.elon.app.chatgptweb

import com.elon.app.BuildConfig
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebAiPrivateTransportCatalogTest {
    @Test
    fun verifiedProductionTransportsAreEnabledAndFallbackSafe() {
        val rows = WebAiPrivateTransportCatalog.describe()
        val values = (0 until rows.length()).map(rows::getJSONObject)

        val enabledIds = values.filter { it.getBoolean("runtime_enabled") }
            .map { it.getString("capability_id") }
            .toSet()
        assertTrue("android_chatgpt_private_conversation_project_directory_v1" in enabledIds)
        assertTrue("android_chatgpt_private_conversation_prefetch_v1" in enabledIds)
        assertTrue("android_chatgpt_conversation_navigation_receipt_reconciliation_v1" in enabledIds)
        assertTrue("android_chatgpt_private_send_dispatch_observer_v1" in enabledIds)
        assertTrue("android_chatgpt_private_stream_observer_v1" in enabledIds)
        assertTrue("android_chatgpt_private_stream_completion_settlement_v1" in enabledIds)
        assertTrue("android_chatgpt_realtime_voice_private_transcript_refresh_v1" in enabledIds)
        assertTrue("android_google_web_private_conversation_directory_v1" in enabledIds)
        assertTrue("android_google_web_conversation_snapshot_cache_v1" in enabledIds)
        assertTrue("android_google_web_private_reply_observer_v1" in enabledIds)
        assertTrue("android_web_ai_background_navigation_continuity_v1" in enabledIds)
        assertTrue("android_web_ai_unified_send_coordinator_v1" in enabledIds)
        assertTrue("android_chatgpt_same_origin_text_transaction_v1" in enabledIds)
        assertTrue("android_chatgpt_attachment_transport_reconciliation_v1" in enabledIds)
        assertTrue("android_chatgpt_native_attachment_progress_v1" in enabledIds)
        assertTrue("android_chatgpt_native_image_asset_gallery_v1" in enabledIds)
        assertTrue("android_chatgpt_native_image_generation_status_v1" in enabledIds)
        assertTrue("android_chatgpt_private_rich_content_native_view_v1" in enabledIds)
        assertTrue("android_chatgpt_private_dictation_transport_v1" in enabledIds)
        assertTrue("android_chatgpt_native_conversation_project_move_v1" in enabledIds)
        assertTrue("android_chatgpt_native_conversation_management_v1" in enabledIds)
        assertTrue("android_chatgpt_realtime_voice_background_overlay_v1" in enabledIds)

        val streamSettlement = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_private_stream_completion_settlement_v1"
        }
        assertEquals("completed", streamSettlement.getString("implementation_status"))
        assertEquals(
            "device_verified_v1_1_1302",
            streamSettlement.getString("verification_status"),
        )
        assertTrue(streamSettlement.getBoolean("production_default"))

        val streamObserver = values.first {
            it.getString("capability_id") == "android_chatgpt_private_stream_observer_v1"
        }
        assertEquals("completed", streamObserver.getString("implementation_status"))
        assertEquals(
            "device_verified_completion_v1_1_1302_and_structural_sparse_watchdog_v1_1_1310",
            streamObserver.getString("verification_status"),
        )
        assertTrue(streamObserver.getBoolean("production_default"))
        assertEquals(
            "official_dom_stream_snapshot",
            streamObserver.getString("fallback"),
        )

        values.forEach { row ->
            if (row.getString("capability_id") !in setOf(
                    "android_chatgpt_same_origin_text_transaction_v1",
                    "android_chatgpt_private_dictation_transport_v1",
                )
            ) {
                assertFalse(row.getBoolean("direct_post_enabled"))
            }
            assertTrue(row.getBoolean("official_page_authoritative"))
            assertTrue(row.getString("fallback").isNotBlank())
        }

        val voiceRefresh = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_realtime_voice_private_transcript_refresh_v1"
        }
        assertEquals("completed", voiceRefresh.getString("implementation_status"))
        assertEquals(
            "targeted_tests_passed_device_pending",
            voiceRefresh.getString("verification_status"),
        )
        assertEquals(
            "retained_native_transcript_and_official_dom_snapshot",
            voiceRefresh.getString("fallback"),
        )
        assertEquals(
            "event_first_active_same_origin_get_current_conversation_reconciliation",
            voiceRefresh.getString("request_mode"),
        )
        assertEquals(
            "single_flight_timeout_cooldown_circuit_breaker_and_sparse_dom_watchdog",
            voiceRefresh.getString("health_policy"),
        )

        val privateDictation = values.first {
            it.getString("capability_id") == "android_chatgpt_private_dictation_transport_v1"
        }
        assertEquals(
            "completed",
            privateDictation.getString("implementation_status"),
        )
        assertTrue(privateDictation.getBoolean("production_default"))
        assertEquals(
            BuildConfig.CHATGPT_PRIVATE_DICTATION_ENABLED,
            privateDictation.getBoolean("runtime_enabled"),
        )
        assertEquals(
            BuildConfig.CHATGPT_PRIVATE_DICTATION_ENABLED,
            privateDictation.getBoolean("direct_post_enabled"),
        )
        assertEquals(
            "same_origin_page_local_media_recorder_buffered_transcription",
            privateDictation.getString("request_mode"),
        )
        assertEquals(
            "none_explicit_long_press_shared_work_mode_selection",
            privateDictation.getString("fallback"),
        )

        val projectMove = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_native_conversation_project_move_v1"
        }
        assertEquals(
            "completed",
            projectMove.getString("implementation_status"),
        )
        assertEquals(
            "device_round_trip_v1_1_1493_adapter_239_exact_two_writes_restored",
            projectMove.getString("verification_status"),
        )
        assertTrue(projectMove.getBoolean("production_default"))
        assertTrue(projectMove.getBoolean("runtime_enabled"))
        assertEquals(
            "cached_native_destination_then_exact_official_dom_activation_and_optional_confirmation",
            projectMove.getString("request_mode"),
        )
        assertEquals(
            "official_conversation_project_menu",
            projectMove.getString("fallback"),
        )

        val conversationManagement = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_native_conversation_management_v1"
        }
        assertEquals("completed", conversationManagement.getString("implementation_status"))
        assertEquals(
            "device_pin_round_trip_v1_1_1399_adapter_218_other_mutations_pending",
            conversationManagement.getString("verification_status"),
        )
        assertTrue(conversationManagement.getBoolean("production_default"))
        assertTrue(conversationManagement.getBoolean("runtime_enabled"))
        assertFalse(conversationManagement.getBoolean("direct_post_enabled"))
        assertEquals(
            "official_conversation_options",
            conversationManagement.getString("fallback"),
        )

        val navigationReceipt = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_conversation_navigation_receipt_reconciliation_v1"
        }
        assertEquals("completed", navigationReceipt.getString("implementation_status"))
        assertEquals(
            "device_verified_v1_1_1399_adapter_218",
            navigationReceipt.getString("verification_status"),
        )
        assertTrue(navigationReceipt.getBoolean("production_default"))
        assertTrue(navigationReceipt.getBoolean("runtime_enabled"))
        assertFalse(navigationReceipt.getBoolean("direct_post_enabled"))

        val nativeDictation = values.first {
            it.getString("capability_id") == "android_chatgpt_native_dictation_v1"
        }
        assertEquals(
            "completed",
            nativeDictation.getString("implementation_status"),
        )
        assertEquals(
            "device_verified_v1_1_1483_explicit_private_and_work_mode_selection",
            nativeDictation.getString("verification_status"),
        )
        assertEquals(
            "none_explicit_long_press_selection_only",
            nativeDictation.getString("fallback"),
        )

        val googleSnapshotCache = values.first {
            it.getString("capability_id") ==
                "android_google_web_conversation_snapshot_cache_v1"
        }
        assertEquals("completed", googleSnapshotCache.getString("implementation_status"))
        assertEquals(
            "device_verified_cache_first_then_official_navigation",
            googleSnapshotCache.getString("verification_status"),
        )
        assertTrue(googleSnapshotCache.getBoolean("production_default"))
        assertEquals(
            "validated_conversation_url_30d_128_items_24mib",
            googleSnapshotCache.getString("health_policy"),
        )
        assertEquals(
            "official_webview_navigation",
            googleSnapshotCache.getString("fallback"),
        )

        val googleReplyObserver = values.first {
            it.getString("capability_id") ==
                "android_google_web_private_reply_observer_v1"
        }
        assertEquals("completed", googleReplyObserver.getString("implementation_status"))
        assertEquals(
            "device_verified_stream_to_completion_v1_1_1303",
            googleReplyObserver.getString("verification_status"),
        )
        assertTrue(googleReplyObserver.getBoolean("production_default"))
        assertEquals(
            "fast_initial_probe_then_sparse_stream_watchdog_and_dom_reconciliation",
            googleReplyObserver.getString("health_policy"),
        )
        assertEquals(
            "official_dom_reply_snapshot",
            googleReplyObserver.getString("fallback"),
        )

        val voiceOverlay = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_realtime_voice_background_overlay_v1"
        }
        assertEquals("completed", voiceOverlay.getString("implementation_status"))
        assertEquals(
            "device_handoff_verified_manual_overlay_actions_pending",
            voiceOverlay.getString("verification_status"),
        )

        val sendCoordinator = values.first {
            it.getString("capability_id") == "android_web_ai_unified_send_coordinator_v1"
        }
        assertEquals("completed", sendCoordinator.getString("implementation_status"))
        assertEquals(
            "all_send_entry_points_targeted_tests_passed_device_pending",
            sendCoordinator.getString("verification_status"),
        )
        assertTrue(sendCoordinator.getBoolean("production_default"))
        assertEquals(
            "single_owner_command_ledger_with_official_page_transport",
            sendCoordinator.getString("request_mode"),
        )
        assertEquals(
            "official_page_reconciliation_without_automatic_write_replay",
            sendCoordinator.getString("fallback"),
        )

        val textTransaction = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_same_origin_text_transaction_v1"
        }
        assertEquals("completed", textTransaction.getString("implementation_status"))
        assertEquals(
            "device_verified_v1_1_1365_adapter_206",
            textTransaction.getString("verification_status"),
        )
        assertTrue(textTransaction.getBoolean("production_default"))
        assertEquals(
            BuildConfig.CHATGPT_PRIVATE_TEXT_TRANSACTIONS_ENABLED,
            textTransaction.getBoolean("runtime_enabled"),
        )
        assertEquals(
            BuildConfig.CHATGPT_PRIVATE_TEXT_TRANSACTIONS_ENABLED,
            textTransaction.getBoolean("direct_post_enabled"),
        )
        assertEquals(
            "immediate_official_page_transaction_without_write_replay",
            textTransaction.getString("fallback"),
        )

        val attachmentTransport = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_attachment_transport_reconciliation_v1"
        }
        assertEquals("completed", attachmentTransport.getString("implementation_status"))
        assertEquals(
            "device_verified_v1_1_1373_adapter_207",
            attachmentTransport.getString("verification_status"),
        )
        assertTrue(attachmentTransport.getBoolean("production_default"))
        assertTrue(attachmentTransport.getBoolean("runtime_enabled"))
        assertFalse(attachmentTransport.getBoolean("direct_post_enabled"))
        assertEquals(
            "official_dom_attachment_snapshot_and_bounded_timeout",
            attachmentTransport.getString("fallback"),
        )

        val attachmentProgress = values.first {
            it.getString("capability_id") == "android_chatgpt_native_attachment_progress_v1"
        }
        assertEquals(
            "completed",
            attachmentProgress.getString("implementation_status"),
        )
        assertEquals(
            "device_verified_v1_1_1491_adapter_239_visible_progress_and_restore",
            attachmentProgress.getString("verification_status"),
        )
        assertTrue(attachmentProgress.getBoolean("production_default"))
        assertTrue(attachmentProgress.getBoolean("runtime_enabled"))
        assertFalse(attachmentProgress.getBoolean("direct_post_enabled"))
        assertEquals(
            "indeterminate_native_status_and_official_dom_attachment_snapshot",
            attachmentProgress.getString("fallback"),
        )

        val imageGallery = values.first {
            it.getString("capability_id") == "android_chatgpt_native_image_asset_gallery_v1"
        }
        assertEquals("completed", imageGallery.getString("implementation_status"))
        assertEquals(
            "device_verified_v1_1_1375_adapter_208",
            imageGallery.getString("verification_status"),
        )
        assertTrue(imageGallery.getBoolean("production_default"))
        assertTrue(imageGallery.getBoolean("runtime_enabled"))
        assertFalse(imageGallery.getBoolean("direct_post_enabled"))
        assertEquals("official_images_page", imageGallery.getString("fallback"))

        val imageGenerationStatus = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_native_image_generation_status_v1"
        }
        assertEquals("completed", imageGenerationStatus.getString("implementation_status"))
        assertEquals(
            "targeted_lifecycle_tests_passed_device_ui_pending",
            imageGenerationStatus.getString("verification_status"),
        )
        assertTrue(imageGenerationStatus.getBoolean("production_default"))
        assertTrue(imageGenerationStatus.getBoolean("runtime_enabled"))
        assertFalse(imageGenerationStatus.getBoolean("direct_post_enabled"))
        assertEquals(
            "official_composer_and_images_page",
            imageGenerationStatus.getString("fallback"),
        )

        val privateResponseReadAloud = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_private_response_read_aloud_v1"
        }
        assertEquals("completed", privateResponseReadAloud.getString("implementation_status"))
        assertEquals(
            "device_verified_v1_1_1498_adapter_241_stream_start_stop",
            privateResponseReadAloud.getString("verification_status"),
        )
        assertTrue(privateResponseReadAloud.getBoolean("production_default"))
        assertTrue(privateResponseReadAloud.getBoolean("runtime_enabled"))
        assertFalse(privateResponseReadAloud.getBoolean("direct_post_enabled"))
        assertEquals(
            "official_dom_read_aloud_or_manual_official_page",
            privateResponseReadAloud.getString("fallback"),
        )

        val officialResponseReadAloud = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_official_response_read_aloud_bridge_v1"
        }
        assertEquals(
            "implemented_device_pending",
            officialResponseReadAloud.getString("implementation_status"),
        )
        assertEquals(
            "live_official_control_discovered_and_targeted_semantic_menu_tests_passed_device_audio_pending",
            officialResponseReadAloud.getString("verification_status"),
        )
        assertFalse(officialResponseReadAloud.getBoolean("production_default"))
        assertTrue(officialResponseReadAloud.getBoolean("runtime_enabled"))
        assertFalse(officialResponseReadAloud.getBoolean("direct_post_enabled"))
        assertEquals(
            "manual_official_page_or_explicit_system_read_aloud_selection",
            officialResponseReadAloud.getString("fallback"),
        )

        val systemResponseReadAloud = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_native_response_read_aloud_v1"
        }
        assertEquals(
            "implemented_device_pending",
            systemResponseReadAloud.getString("implementation_status"),
        )
        assertEquals(
            "device_audio_start_user_confirmed_targeted_stop_and_failure_tests_passed_stop_acceptance_pending",
            systemResponseReadAloud.getString("verification_status"),
        )
        assertFalse(systemResponseReadAloud.getBoolean("production_default"))
        assertTrue(systemResponseReadAloud.getBoolean("runtime_enabled"))
        assertFalse(systemResponseReadAloud.getBoolean("direct_post_enabled"))
        assertEquals(
            "none_explicit_user_selection_only",
            systemResponseReadAloud.getString("fallback"),
        )

        val richContent = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_private_rich_content_native_view_v1"
        }
        assertEquals("completed", richContent.getString("implementation_status"))
        assertEquals(
            "device_structural_verified_v1_1_1379_parser_contract_and_native_finance_chart_preview",
            richContent.getString("verification_status"),
        )
        assertTrue(richContent.getBoolean("production_default"))
        assertTrue(richContent.getBoolean("runtime_enabled"))
        assertFalse(richContent.getBoolean("direct_post_enabled"))
        assertEquals(
            "official_webview_rich_content",
            richContent.getString("fallback"),
        )
    }

    @Test
    fun researchAndUnsafeShortcutsStayDisabled() {
        val rows = WebAiPrivateTransportCatalog.describe()
        val values = (0 until rows.length()).map(rows::getJSONObject)
        val byId = values.associateBy { it.getString("capability_id") }

        val googleSend = requireNotNull(byId["android_google_web_direct_send_decision_v1"])
        assertEquals("audited_no_safe_gain", googleSend.getString("implementation_status"))
        assertFalse(googleSend.getBoolean("runtime_enabled"))

        val googlePrefetch = requireNotNull(
            byId["android_google_web_conversation_prefetch_decision_v1"],
        )
        assertEquals(
            "deferred_no_observed_endpoint",
            googlePrefetch.getString("implementation_status"),
        )
        assertFalse(googlePrefetch.getBoolean("runtime_enabled"))

        val voiceReuse = requireNotNull(
            byId["android_chatgpt_realtime_voice_session_reuse_decision_v1"],
        )
        assertEquals("audited_no_safe_gain", voiceReuse.getString("implementation_status"))
        assertFalse(voiceReuse.getBoolean("runtime_enabled"))

        val privateVoiceResearch = requireNotNull(
            byId["android_chatgpt_web_private_voice_bootstrap_research_v1"],
        )
        assertEquals(
            "research_completed",
            privateVoiceResearch.getString("implementation_status"),
        )
        assertFalse(privateVoiceResearch.getBoolean("production_default"))
        assertEquals(
            BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED,
            privateVoiceResearch.getBoolean("runtime_enabled"),
        )
        assertEquals(
            "official_page_created_webrtc",
            privateVoiceResearch.getString("fallback"),
        )

        val nativeVoiceRelay = requireNotNull(
            byId["android_chatgpt_web_private_voice_native_relay_v1"],
        )
        assertEquals(
            "completed",
            nativeVoiceRelay.getString("implementation_status"),
        )
        assertEquals(
            "device_native_single_audio_and_data_channel_verified",
            nativeVoiceRelay.getString("verification_status"),
        )
        assertTrue(nativeVoiceRelay.getBoolean("production_default"))
        assertEquals(
            BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED,
            nativeVoiceRelay.getBoolean("runtime_enabled"),
        )
        assertEquals(
            "official_page_created_webrtc",
            nativeVoiceRelay.getString("fallback"),
        )

        val nativeVoiceTranscript = requireNotNull(
            byId["android_chatgpt_realtime_voice_data_channel_transcript_v1"],
        )
        assertEquals(
            "completed",
            nativeVoiceTranscript.getString("implementation_status"),
        )
        assertEquals(
            "device_private_delta_shape_observed_native_peer_connected_targeted_tests_passed",
            nativeVoiceTranscript.getString("verification_status"),
        )
        assertEquals(
            "bounded_utf8_delta_decoder_deduplicated_in_memory_stream",
            nativeVoiceTranscript.getString("health_policy"),
        )
        assertTrue(nativeVoiceTranscript.getBoolean("production_default"))
        assertEquals(
            BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED,
            nativeVoiceTranscript.getBoolean("runtime_enabled"),
        )
        assertEquals(
            "private_conversation_refresh_and_official_dom_snapshot",
            nativeVoiceTranscript.getString("fallback"),
        )

        values.filter { it.getString("implementation_status") == "research_only" }
            .forEach { row ->
                assertFalse(row.getBoolean("production_default"))
                if (row.getBoolean("runtime_enabled")) {
                    assertEquals("chatgpt", row.getString("provider"))
                    assertTrue(BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED)
                }
            }
    }
}
