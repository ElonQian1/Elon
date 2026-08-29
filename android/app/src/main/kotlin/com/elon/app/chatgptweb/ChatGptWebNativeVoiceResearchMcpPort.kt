package com.elon.app.chatgptweb

import com.elon.app.BuildConfig
import com.elon.app.WebChatSocialMcpPort
import org.json.JSONArray
import org.json.JSONObject

/** Research controls layered onto the existing ChatGPT MCP surface. */
internal class ChatGptWebNativeVoiceResearchMcpPort(
    private val delegate: WebChatSocialMcpPort,
    private val startNative: ((ChatGptWebNativeVoiceState) -> Unit) -> Boolean,
    private val muteNative: (Boolean) -> Boolean,
    private val stopNative: () -> Unit,
    private val currentState: (() -> ChatGptWebNativeVoiceState)? = null,
    private val enabled: Boolean = BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED,
) : WebChatSocialMcpPort {
    @Volatile
    private var state = ChatGptWebNativeVoiceState(ChatGptWebNativeVoicePhase.IDLE)

    override fun uiState(): JSONObject = delegate.uiState().apply {
        put(STATE_KEY, stateJson())
        if (enabled) {
            val actions = optJSONArray("available_actions") ?: JSONArray()
            RESEARCH_ACTIONS.filterNot { actions.containsString(it) }.forEach(actions::put)
            put("available_actions", actions)
        }
    }

    override fun control(args: JSONObject): JSONObject {
        val action = args.optString("action").trim().lowercase()
        if (action !in RESEARCH_ACTIONS) return delegate.control(args)
        if (!enabled) {
            return result(action, false, "native_research_disabled")
        }
        val accepted = when (action) {
            ACTION_START -> {
                state = ChatGptWebNativeVoiceState(ChatGptWebNativeVoicePhase.BOOTSTRAPPING)
                startNative(::acceptState)
            }
            ACTION_MUTE -> muteNative(args.optBoolean("muted", true))
            ACTION_STOP -> {
                stopNative()
                state = ChatGptWebNativeVoiceState(ChatGptWebNativeVoicePhase.CLOSED)
                true
            }
            else -> false
        }
        if (!accepted && action == ACTION_START) {
            state = ChatGptWebNativeVoiceState(
                phase = ChatGptWebNativeVoicePhase.FAILED,
                code = "start_rejected",
            )
        }
        return result(action, accepted, if (accepted) null else "native_research_unavailable")
    }

    private fun acceptState(value: ChatGptWebNativeVoiceState) {
        state = value
    }

    private fun result(action: String, accepted: Boolean, error: String?): JSONObject =
        uiState()
            .put("control_ok", accepted)
            .put("action", action)
            .put("error", error ?: JSONObject.NULL)

    private fun stateJson(): JSONObject {
        val observed = currentState?.invoke() ?: state
        return JSONObject()
            .put("enabled", enabled)
            .put("phase", observed.phase.name.lowercase())
            .put("remote_audio", observed.remoteAudio)
            .put("data_channel_open", observed.dataChannelOpen)
            .put("data_channel_message_count", observed.dataChannelMessageCount)
            .put("transcript_event_count", observed.transcriptEventCount)
            .put("official_media_suspended", observed.officialMediaSuspended)
            .put("official_peer_released", observed.officialPeerReleased)
            .put("code", observed.code ?: JSONObject.NULL)
    }

    private fun JSONArray.containsString(value: String): Boolean =
        (0 until length()).any { optString(it) == value }

    private companion object {
        const val STATE_KEY = "private_voice_native_research"
        const val ACTION_START = "chatgpt_start_private_voice_native_research"
        const val ACTION_MUTE = "chatgpt_mute_private_voice_native_research"
        const val ACTION_STOP = "chatgpt_stop_private_voice_native_research"
        val RESEARCH_ACTIONS = setOf(ACTION_START, ACTION_MUTE, ACTION_STOP)
    }
}
