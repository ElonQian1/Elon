package com.elon.app

internal object WebChatRealtimeVoiceEndPolicy {
    fun resolve(controls: List<WebChatConsumerControlDescriptor>): WebChatConsumerControl? =
        controls.asSequence()
            .map(WebChatConsumerControlDescriptor::control)
            .filter { it.enabled && it.inViewport }
            .firstOrNull { control ->
                control.semantic in END_SEMANTICS && END_LABEL.containsMatchIn(control.label.trim())
            }

    private val END_SEMANTICS = setOf("close", "stop", "action")
    private val END_LABEL = Regex(
        "(?:end|exit|leave|stop|close|hang\\s*up).*(?:voice|call)|" +
            "(?:voice|call).*(?:end|exit|leave|stop|close)|" +
            "挂断|结束.*(?:语音|通话)|退出.*语音|离开.*语音|关闭.*语音",
        RegexOption.IGNORE_CASE,
    )
}
