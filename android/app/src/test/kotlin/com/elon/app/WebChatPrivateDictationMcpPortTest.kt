package com.elon.app

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatPrivateDictationMcpPortTest {
    @Test
    fun reportsPrivateStateWithoutTranscriptContents() {
        val privatePort = FakePrivatePort(
            WebChatNativeDictationState(WebChatNativeDictationPhase.LISTENING),
        )
        val port = WebChatPrivateDictationMcpPort(
            delegate = FakeSocialPort(),
            dictation = privatePort,
            readyCheck = { true },
            enabled = true,
        )

        val state = port.uiState().getJSONObject("private_dictation_native")

        assertTrue(state.getBoolean("enabled"))
        assertTrue(state.getBoolean("ready"))
        assertTrue(state.getBoolean("active"))
        assertEquals("listening", state.getString("phase"))
        assertEquals(setOf("enabled", "ready", "active", "phase"), state.keys().asSequence().toSet())
    }

    @Test
    fun delegatesControlUnchanged() {
        val port = WebChatPrivateDictationMcpPort(
            delegate = FakeSocialPort(),
            dictation = FakePrivatePort(WebChatNativeDictationState()),
            readyCheck = { false },
            enabled = false,
        )

        assertEquals("probe", port.control(JSONObject().put("action", "probe")).getString("action"))
    }

    private class FakeSocialPort : WebChatSocialMcpPort {
        override fun uiState() = JSONObject().put("surface", "fake")
        override fun control(args: JSONObject) = JSONObject().put("action", args.optString("action"))
    }

    private class FakePrivatePort(
        private val current: WebChatNativeDictationState,
    ) : WebChatPrivateDictationPort {
        override fun ready() = true
        override fun start(onStateChanged: (WebChatNativeDictationState) -> Unit) = true
        override fun submit() = true
        override fun cancel() = true
        override fun state() = current
        override fun destroy() = Unit
    }
}
