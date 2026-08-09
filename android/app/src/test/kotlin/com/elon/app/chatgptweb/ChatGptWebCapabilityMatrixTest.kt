package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebCapabilityMatrixTest {
    @Test
    fun reportsReadyChatAndAdaptiveReviewWithoutTreatingGenericControlsAsBlocking() {
        val matrix = ChatGptWebCapabilityMatrix.build(
            snapshot = snapshot(
                setOf(
                    ChatGptWebCapabilityId.DRAFT_SYNC,
                    ChatGptWebCapabilityId.CURRENT_CONVERSATION,
                ),
            ),
            manifest = manifest("healthy", "action"),
            bridgeState = ChatGptWebPageAdapter.State.READY,
            mode = ChatGptWebModeController.Mode.NATIVE,
        )

        assertTrue(matrix.getBoolean("ready_for_chat"))
        assertTrue(matrix.getBoolean("ready_for_mcp"))
        assertEquals(ChatGptWebPageAdapter.ADAPTER_VERSION, matrix.getInt("adapter_version"))
        assertEquals(0, matrix.getJSONArray("blocking_gaps").length())
        assertTrue(matrix.getJSONObject("adaptation_review").getBoolean("required"))
        assertEquals(1, matrix.getJSONObject("manifest").getInt("generic_control_count"))
        assertEquals(1, matrix.getJSONObject("manifest").getInt("native_menu_control_count"))
        assertEquals(0, matrix.getJSONObject("manifest").getInt("official_fallback_control_count"))
    }

    @Test
    fun reportsUnknownOfficialCapabilitiesAndSemanticsForTheNextAdapterPass() {
        val matrix = ChatGptWebCapabilityMatrix.build(
            snapshot = snapshot(setOf("future_official_capability")),
            manifest = manifest("partial", "future_semantic"),
            bridgeState = ChatGptWebPageAdapter.State.READY,
            mode = ChatGptWebModeController.Mode.WEB,
        )

        assertEquals(
            "future_official_capability",
            matrix.getJSONArray("unknown_capabilities").getString(0),
        )
        assertEquals("future_semantic", matrix.getJSONArray("unknown_semantics").getString(0))
        assertEquals("manifest_partial", matrix.getJSONArray("blocking_gaps").getString(0))
        assertFalse(matrix.getBoolean("ready_for_chat"))
    }

    @Test
    fun reportsAuthenticationAndBridgeFailuresAsBlocking() {
        val snapshot = snapshot(emptySet()).copy(authenticated = false, composerReady = false)
        val matrix = ChatGptWebCapabilityMatrix.build(
            snapshot = snapshot,
            manifest = null,
            bridgeState = ChatGptWebPageAdapter.State.CONNECTING,
            mode = ChatGptWebModeController.Mode.WEB,
        )

        val gaps = matrix.getJSONArray("blocking_gaps")
        assertEquals(3, gaps.length())
        assertEquals("bridge_not_ready", gaps.getString(0))
        assertEquals("not_authenticated", gaps.getString(1))
        assertEquals("manifest_unavailable", gaps.getString(2))
        assertFalse(matrix.getBoolean("ready_for_mcp"))
    }

    @Test
    fun reportsKnownControlsThatOnlyHaveTheOfficialFallback() {
        val matrix = ChatGptWebCapabilityMatrix.build(
            snapshot = snapshot(emptySet()),
            manifest = manifest("healthy", "sources").copy(
                controls = manifest("healthy", "sources").controls.map {
                    it.copy(region = ChatGptWebUiRegion.SUGGESTIONS)
                },
            ),
            bridgeState = ChatGptWebPageAdapter.State.READY,
            mode = ChatGptWebModeController.Mode.NATIVE,
        )

        assertEquals(1, matrix.getJSONObject("manifest").getInt("official_fallback_control_count"))
        assertEquals(
            "official_fallback",
            matrix.getJSONArray("control_coverage").getJSONObject(0).getString("presentation"),
        )
        assertEquals(
            "official_fallback_controls_present",
            matrix.getJSONObject("adaptation_review").getJSONArray("reasons").getString(0),
        )
    }

    private fun snapshot(capabilities: Set<String>) = ChatGptWebSnapshot(
        title = "Work",
        url = "https://chatgpt.com/",
        draft = "",
        messages = emptyList(),
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "5.6 Sol",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(capabilities),
    )

    private fun manifest(compatibility: String, semantic: String) = ChatGptWebUiManifest(
        version = 3,
        pageKind = "home",
        title = "Work",
        compatibility = compatibility,
        controls = listOf(
            ChatGptWebUiControl(
                id = "control_demo",
                semantic = semantic,
                label = "Demo",
                region = ChatGptWebUiRegion.OVERLAY,
                role = "button",
                enabled = true,
                selected = false,
            ),
        ),
    )
}
