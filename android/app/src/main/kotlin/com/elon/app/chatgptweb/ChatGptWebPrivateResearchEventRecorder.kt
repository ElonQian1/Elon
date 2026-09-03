package com.elon.app.chatgptweb

import android.os.SystemClock
import com.elon.app.BuildConfig
import com.elon.app.DebugTraceStore
import java.util.concurrent.ConcurrentHashMap

internal object ChatGptWebPrivateResearchEventRecorder {
    private const val NETWORK_ACTION = "research_network_observation"
    private const val VOICE_ACTION = "research_voice_observation"
    private const val VOICE_WINDOW_MS = 2 * 60 * 1000L
    private const val MAX_RESOURCE_SHAPES = 64
    private val NETWORK_DETAIL = Regex("^[A-Za-z0-9._:/|{}-]{1,160}$")
    @Volatile private var voiceWindowDeadlineElapsedMs = 0L
    private val resourceShapes = ConcurrentHashMap.newKeySet<String>()

    fun record(event: ChatGptWebEvent): Boolean {
        if (event is ChatGptWebEvent.AttachmentTransport) {
            if (BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED) {
                DebugTraceStore.record(
                    phase = "chatgpt_attachment_transport_observation",
                    details = mapOf(
                        "version" to event.evidence.version,
                        "sequence" to event.evidence.sequence,
                        "state" to event.evidence.state.wireValue,
                        "completed_count" to event.evidence.completedCount,
                    ),
                )
            }
            return false
        }
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
                    if (it.channel == "window-start") beginVoiceWindow()
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

    fun recordResourceRequest(method: String, url: String, contentType: String?): Boolean {
        if (!BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED) return false
        if (SystemClock.elapsedRealtime() > voiceWindowDeadlineElapsedMs) return false
        val detail = ChatGptWebResearchResourceShape.from(method, url, contentType) ?: return false
        if (resourceShapes.size >= MAX_RESOURCE_SHAPES || !resourceShapes.add(detail)) return false
        val observation = ChatGptWebRealtimeVoiceResearchObservation.parse(detail) ?: return false
        DebugTraceStore.record(
            phase = "chatgpt_private_voice_research_observation",
            details = observation.traceDetails(),
        )
        return true
    }

    fun beginVoiceWindow() {
        if (!BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED) return
        resourceShapes.clear()
        voiceWindowDeadlineElapsedMs = SystemClock.elapsedRealtime() + VOICE_WINDOW_MS
    }
}
