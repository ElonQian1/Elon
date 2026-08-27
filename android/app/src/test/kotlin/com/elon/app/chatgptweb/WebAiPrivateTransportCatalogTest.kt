package com.elon.app.chatgptweb

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
        assertTrue("android_chatgpt_private_send_dispatch_observer_v1" in enabledIds)
        assertTrue("android_chatgpt_private_stream_observer_v1" in enabledIds)
        assertTrue("android_chatgpt_private_stream_completion_settlement_v1" in enabledIds)
        assertTrue("android_chatgpt_realtime_voice_private_transcript_refresh_v1" in enabledIds)
        assertTrue("android_google_web_private_conversation_directory_v1" in enabledIds)
        assertTrue("android_google_web_conversation_snapshot_cache_v1" in enabledIds)
        assertTrue("android_google_web_private_reply_observer_v1" in enabledIds)
        assertTrue("android_web_ai_background_navigation_continuity_v1" in enabledIds)
        assertTrue("android_web_ai_unified_send_coordinator_v1" in enabledIds)
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
            assertFalse(row.getBoolean("direct_post_enabled"))
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
            "command_ledger_targeted_tests_passed_device_pending",
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
        assertFalse(privateVoiceResearch.getBoolean("runtime_enabled"))
        assertEquals(
            "official_page_created_webrtc",
            privateVoiceResearch.getString("fallback"),
        )

        val nativeVoiceRelay = requireNotNull(
            byId["android_chatgpt_web_private_voice_native_relay_v1"],
        )
        assertEquals("planned", nativeVoiceRelay.getString("implementation_status"))
        assertFalse(nativeVoiceRelay.getBoolean("production_default"))
        assertFalse(nativeVoiceRelay.getBoolean("runtime_enabled"))
        assertEquals(
            "official_page_created_webrtc",
            nativeVoiceRelay.getString("fallback"),
        )

        values.filter { it.getString("implementation_status") == "research_only" }
            .forEach { row ->
                assertFalse(row.getBoolean("production_default"))
                assertFalse(row.getBoolean("runtime_enabled"))
            }
    }
}
