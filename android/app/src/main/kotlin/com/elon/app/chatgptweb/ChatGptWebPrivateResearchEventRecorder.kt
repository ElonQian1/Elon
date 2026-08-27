package com.elon.app.chatgptweb

import com.elon.app.BuildConfig
import com.elon.app.DebugTraceStore

internal object ChatGptWebPrivateResearchEventRecorder {
    private const val NETWORK_ACTION = "research_network_observation"
    private const val VOICE_ACTION = "research_voice_observation"
    private val NETWORK_DETAIL = Regex("^[A-Za-z0-9._:/|{}-]{1,160}$")

    fun record(event: ChatGptWebEvent): Boolean {
        if (event !is ChatGptWebEvent.CommandResult) return false
        return when (event.action) {
            NETWORK_ACTION -> {
                if (
                    BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED &&
                    event.ok &&
                    NETWORK_DETAIL.matches(event.detail)
                ) {
                    DebugTraceStore.record(
                        phase = "chatgpt_private_research_observation",
                        details = mapOf("summary" to event.detail),
                    )
                }
                true
            }
            VOICE_ACTION -> {
                val observation = if (
                    BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED && event.ok
                ) {
                    ChatGptWebRealtimeVoiceResearchObservation.parse(event.detail)
                } else {
                    null
                }
                observation?.let {
                    DebugTraceStore.record(
                        phase = "chatgpt_private_voice_research_observation",
                        details = it.traceDetails(),
                    )
                }
                true
            }
            else -> false
        }
    }
}
