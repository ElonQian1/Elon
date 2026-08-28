package com.elon.app.chatgptweb

import com.elon.app.WebChatSocialMcpPort
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebNativeVoiceResearchMcpPortTest {
    @Test
    fun disabledBuildRejectsResearchWithoutInvokingNativeCallbacks() {
        var invoked = false
        val port = port(
            enabled = false,
            start = { invoked = true; true },
        )

        val result = port.control(action("chatgpt_start_private_voice_native_research"))

        assertFalse(result.getBoolean("control_ok"))
        assertEquals("native_research_disabled", result.getString("error"))
        assertFalse(invoked)
        assertFalse(result.getJSONObject("private_voice_native_research").getBoolean("enabled"))
    }

    @Test
    fun enabledBuildReportsOneAuthoritativeLifecycleState() {
        var observer: ((ChatGptWebNativeVoiceState) -> Unit)? = null
        var muted: Boolean? = null
        var stopped = false
        val port = port(
            enabled = true,
            start = { observer = it; true },
            mute = { muted = it; true },
            stop = { stopped = true },
        )

        val start = port.control(action("chatgpt_start_private_voice_native_research"))
        assertTrue(start.getBoolean("control_ok"))
        assertEquals(
            "bootstrapping",
            start.getJSONObject("private_voice_native_research").getString("phase"),
        )

        observer?.invoke(
            ChatGptWebNativeVoiceState(
                phase = ChatGptWebNativeVoicePhase.CONNECTED,
                remoteAudio = true,
                dataChannelOpen = true,
                officialMediaSuspended = true,
                officialPeerReleased = true,
            ),
        )
        val connected = port.uiState().getJSONObject("private_voice_native_research")
        assertEquals("connected", connected.getString("phase"))
        assertTrue(connected.getBoolean("remote_audio"))
        assertTrue(connected.getBoolean("data_channel_open"))
        assertTrue(connected.getBoolean("official_media_suspended"))
        assertTrue(connected.getBoolean("official_peer_released"))

        assertTrue(
            port.control(
                action("chatgpt_mute_private_voice_native_research").put("muted", true),
            ).getBoolean("control_ok"),
        )
        assertEquals(true, muted)
        assertTrue(
            port.control(action("chatgpt_stop_private_voice_native_research"))
                .getBoolean("control_ok"),
        )
        assertTrue(stopped)
        assertEquals(
            "closed",
            port.uiState().getJSONObject("private_voice_native_research").getString("phase"),
        )
    }

    private fun port(
        enabled: Boolean,
        start: ((ChatGptWebNativeVoiceState) -> Unit) -> Boolean = { true },
        mute: (Boolean) -> Boolean = { true },
        stop: () -> Unit = {},
    ): ChatGptWebNativeVoiceResearchMcpPort = ChatGptWebNativeVoiceResearchMcpPort(
        delegate = FakePort(),
        startNative = start,
        muteNative = mute,
        stopNative = stop,
        enabled = enabled,
    )

    private fun action(value: String): JSONObject = JSONObject().put("action", value)

    private class FakePort : WebChatSocialMcpPort {
        override fun uiState(): JSONObject = JSONObject()
            .put("available_actions", JSONArray().put("state"))

        override fun control(args: JSONObject): JSONObject = JSONObject()
            .put("delegated", args.optString("action"))
    }
}
