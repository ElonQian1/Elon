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
        assertTrue("android_chatgpt_realtime_voice_private_transcript_refresh_v1" in enabledIds)
        assertTrue("android_google_web_private_conversation_directory_v1" in enabledIds)
        assertTrue("android_google_web_conversation_snapshot_cache_v1" in enabledIds)
        assertTrue("android_google_web_private_reply_observer_v1" in enabledIds)
        assertTrue("android_web_ai_background_navigation_continuity_v1" in enabledIds)
        assertTrue("android_chatgpt_realtime_voice_background_overlay_v1" in enabledIds)

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

        val voiceOverlay = values.first {
            it.getString("capability_id") ==
                "android_chatgpt_realtime_voice_background_overlay_v1"
        }
        assertEquals("completed", voiceOverlay.getString("implementation_status"))
        assertEquals(
            "device_handoff_verified_manual_overlay_actions_pending",
            voiceOverlay.getString("verification_status"),
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

        values.filter { it.getString("implementation_status") == "research_only" }
            .forEach { row ->
                assertFalse(row.getBoolean("production_default"))
                assertFalse(row.getBoolean("runtime_enabled"))
            }
    }
}
