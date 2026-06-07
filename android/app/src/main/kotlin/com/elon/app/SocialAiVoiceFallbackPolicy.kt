package com.elon.app

internal object SocialAiVoiceFallbackPolicy {
    fun shouldFallbackToTranscribe(message: String, alreadyUsingFallback: Boolean): Boolean {
        if (alreadyUsingFallback) return false
        val normalized = message.lowercase()
        return REALTIME_FAILURE_MARKERS.any { marker ->
            normalized.contains(marker)
        }
    }

    private val REALTIME_FAILURE_MARKERS = listOf(
        "realtime",
        "real-time",
        "实时通话",
        "websocket",
        "ws_failure",
        "连接 openai",
        "openai realtime",
        "api key",
        "未配置 openai",
        "model",
        "session.update",
        "response.create",
        "unsupported",
        "invalid",
    )
}
