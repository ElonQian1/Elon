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
        val control = matrix.getJSONArray("control_coverage").getJSONObject(0)
        assertEquals("chatgpt_invoke_control", control.getString("mcp_action"))
        assertEquals("control_demo", control.getJSONObject("mcp_arguments").getString("control_id"))
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
    fun activeDictationRemainsAReadyAuthenticatedSessionWithoutAComposer() {
        val dictating = snapshot(setOf(ChatGptWebCapabilityId.DICTATION)).copy(
            composerReady = false,
            dictationActive = true,
        )

        val matrix = ChatGptWebCapabilityMatrix.build(
            snapshot = dictating,
            manifest = manifest("partial", "action"),
            bridgeState = ChatGptWebPageAdapter.State.READY,
            mode = ChatGptWebModeController.Mode.WEB,
        )

        assertTrue(matrix.getBoolean("dictation_active"))
        assertTrue(matrix.getBoolean("ready_for_chat"))
        assertEquals(0, matrix.getJSONArray("blocking_gaps").length())
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
        assertEquals(0, matrix.getJSONObject("manifest").getInt("expected_official_fallback_control_count"))
        assertEquals(1, matrix.getJSONObject("manifest").getInt("unexpected_official_fallback_control_count"))
        assertEquals(
            "official_fallback",
            matrix.getJSONArray("control_coverage").getJSONObject(0).getString("presentation"),
        )
        assertEquals(
            "unexpected_official_fallback_controls_present",
            matrix.getJSONObject("adaptation_review").getJSONArray("reasons").getString(0),
        )
    }

    @Test
    fun acceptsRealtimeVoiceAsAnIntentionalOfficialFallback() {
        val voiceManifest = manifest("healthy", "voice_mode").copy(
            controls = manifest("healthy", "voice_mode").controls.map {
                it.copy(region = ChatGptWebUiRegion.COMPOSER)
            },
        )

        val matrix = ChatGptWebCapabilityMatrix.build(
            snapshot = snapshot(emptySet()),
            manifest = voiceManifest,
            bridgeState = ChatGptWebPageAdapter.State.READY,
            mode = ChatGptWebModeController.Mode.NATIVE,
        )

        val manifest = matrix.getJSONObject("manifest")
        assertEquals(1, manifest.getInt("official_fallback_control_count"))
        assertEquals(1, manifest.getInt("expected_official_fallback_control_count"))
        assertEquals(0, manifest.getInt("unexpected_official_fallback_control_count"))
        assertEquals(
            "expected",
            matrix.getJSONArray("control_coverage").getJSONObject(0)
                .getString("official_fallback_policy"),
        )
        assertFalse(matrix.getJSONObject("adaptation_review").getBoolean("required"))
    }

    @Test
    fun doesNotRequestAdaptationForTheHeaderTitleAlreadyRenderedByNativeUi() {
        val titleManifest = manifest("healthy", "title").copy(
            controls = manifest("healthy", "title").controls.map {
                it.copy(region = ChatGptWebUiRegion.HEADER, label = "工作")
            },
        )

        val matrix = ChatGptWebCapabilityMatrix.build(
            snapshot = snapshot(emptySet()),
            manifest = titleManifest,
            bridgeState = ChatGptWebPageAdapter.State.READY,
            mode = ChatGptWebModeController.Mode.NATIVE,
        )

        assertEquals(0, matrix.getJSONObject("manifest").getInt("official_fallback_control_count"))
        assertEquals(
            "metadata",
            matrix.getJSONArray("control_coverage").getJSONObject(0).getString("presentation"),
        )
        assertFalse(matrix.getJSONObject("adaptation_review").getBoolean("required"))
    }

    @Test
    fun treatsCurrentConversationMenuSemanticsAsAdaptedControls() {
        val controls = listOf("conversation_files", "pin", "archive")
        controls.forEach { semantic ->
            val matrix = ChatGptWebCapabilityMatrix.build(
                snapshot = snapshot(emptySet()),
                manifest = manifest("healthy", semantic),
                bridgeState = ChatGptWebPageAdapter.State.READY,
                mode = ChatGptWebModeController.Mode.NATIVE,
            )

            assertEquals(0, matrix.getJSONObject("manifest").getInt("generic_control_count"))
            assertFalse(matrix.getJSONObject("adaptation_review").getBoolean("required"))
            assertEquals(1, matrix.getJSONObject("observed_semantics").getInt(semantic))
        }
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
