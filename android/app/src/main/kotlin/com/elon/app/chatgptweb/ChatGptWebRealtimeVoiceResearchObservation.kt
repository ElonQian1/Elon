package com.elon.app.chatgptweb

internal data class ChatGptWebRealtimeVoiceResearchObservation(
    val channel: String,
    val detail: String,
) {
    fun traceDetails(): Map<String, String> = mapOf(
        "channel" to channel,
        "summary" to detail,
    )

    companion object {
        private val SAFE_DETAIL = Regex("^[a-z0-9._:/|{}-]{1,160}$")
        private val SENSITIVE_MARKER = Regex(
            "(^|[|._/-])(authorization|bearer|cookie|credential|proof|sdp|candidate|secret|token)([|._/-]|$)",
        )
        private val CHANNELS = setOf(
            "observer-ready",
            "media-request",
            "media-granted",
            "media-error",
            "peer-created",
            "peer-create-offer",
            "peer-create-answer",
            "peer-local-description",
            "peer-remote-description",
            "peer-connection",
            "peer-ice",
            "peer-signaling",
            "peer-track",
            "peer-data-channel",
            "network-start",
            "network-end",
            "network-error",
            "network-shape",
            "socket-start",
            "socket-open",
            "socket-close",
            "socket-error",
        )

        fun parse(value: String): ChatGptWebRealtimeVoiceResearchObservation? {
            if (!SAFE_DETAIL.matches(value) || SENSITIVE_MARKER.containsMatchIn(value)) return null
            val parts = value.split('|')
            if (parts.size !in 2..8 || parts[0] != "v1") return null
            val channel = parts[1]
            if (channel !in CHANNELS) return null
            return ChatGptWebRealtimeVoiceResearchObservation(channel, value)
        }
    }
}
