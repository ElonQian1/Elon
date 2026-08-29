package com.elon.app

internal sealed interface WebChatRealtimeVoicePauseControlDecision {
    data object AlreadyApplied : WebChatRealtimeVoicePauseControlDecision
    data class Invoke(val controlId: String) : WebChatRealtimeVoicePauseControlDecision
    data object RefreshControls : WebChatRealtimeVoicePauseControlDecision
}

internal object WebChatRealtimeVoicePauseControlPolicy {
    fun decide(
        controls: List<WebChatConsumerControlDescriptor>,
        paused: Boolean,
    ): WebChatRealtimeVoicePauseControlDecision {
        val mute = controls.firstEnabled(SEMANTIC_MUTE)
        val unmute = controls.firstEnabled(SEMANTIC_UNMUTE)
        return when {
            paused && unmute != null -> WebChatRealtimeVoicePauseControlDecision.AlreadyApplied
            !paused && mute != null -> WebChatRealtimeVoicePauseControlDecision.AlreadyApplied
            paused && mute != null -> WebChatRealtimeVoicePauseControlDecision.Invoke(mute.control.id)
            !paused && unmute != null -> WebChatRealtimeVoicePauseControlDecision.Invoke(unmute.control.id)
            else -> WebChatRealtimeVoicePauseControlDecision.RefreshControls
        }
    }

    private fun List<WebChatConsumerControlDescriptor>.firstEnabled(semantic: String) =
        firstOrNull { it.control.semantic == semantic && it.control.enabled }

    private const val SEMANTIC_MUTE = "voice_mute"
    private const val SEMANTIC_UNMUTE = "voice_unmute"
}

internal class WebChatRealtimeVoicePauseRouter(
    private val managedVoice: WebChatManagedRealtimeVoiceLaunchCoordinator,
    private val officialVoice: WebChatRealtimeVoicePauseController,
    private val onManagedApplied: (paused: Boolean, detail: String) -> Unit,
) {
    fun request(paused: Boolean, source: WebChatRealtimeVoiceBackgroundControlSource) {
        if (managedVoice.setMutedIfActive(paused)) {
            onManagedApplied(paused, if (paused) "原生麦克风已暂停" else "原生麦克风已恢复")
        } else {
            officialVoice.request(paused, source)
        }
    }
}

internal class WebChatRealtimeVoicePauseController(
    private val consumerPort: () -> WebChatConsumerPort?,
    private val schedule: (Runnable, Long) -> Unit,
    private val onCompleted: (paused: Boolean, detail: String) -> Unit,
    private val onFailed: (detail: String) -> Unit,
) {
    private var generation = 0

    fun request(paused: Boolean, source: WebChatRealtimeVoiceBackgroundControlSource) {
        generation += 1
        attempt(generation, paused, source, attempt = 0, invoked = false)
    }

    fun reset() {
        generation += 1
    }

    private fun attempt(
        expectedGeneration: Int,
        paused: Boolean,
        source: WebChatRealtimeVoiceBackgroundControlSource,
        attempt: Int,
        invoked: Boolean,
    ) {
        if (expectedGeneration != generation) return
        val port = consumerPort()
        if (port == null) {
            fail("网页语音控制尚未连接")
            return
        }
        when (val decision = WebChatRealtimeVoicePauseControlPolicy.decide(port.state().controls, paused)) {
            WebChatRealtimeVoicePauseControlDecision.AlreadyApplied -> complete(paused, source)
            is WebChatRealtimeVoicePauseControlDecision.Invoke -> {
                if (invoked) {
                    port.requestControls()
                    retryOrFail(expectedGeneration, paused, source, attempt, invoked = true)
                    return
                }
                val result = port.invokeControl(decision.controlId, userConfirmed = false)
                if (!result.accepted) {
                    fail("官网没有接受语音${if (paused) "暂停" else "继续"}操作")
                    return
                }
                port.requestControls()
                retryOrFail(expectedGeneration, paused, source, attempt, invoked = true)
            }
            WebChatRealtimeVoicePauseControlDecision.RefreshControls -> {
                port.requestControls()
                retryOrFail(expectedGeneration, paused, source, attempt, invoked)
            }
        }
    }

    private fun retryOrFail(
        expectedGeneration: Int,
        paused: Boolean,
        source: WebChatRealtimeVoiceBackgroundControlSource,
        attempt: Int,
        invoked: Boolean,
    ) {
        if (attempt >= MAX_ATTEMPTS) {
            fail("未能确认官网语音${if (paused) "已暂停" else "已继续"}")
            return
        }
        schedule(Runnable {
            attempt(expectedGeneration, paused, source, attempt + 1, invoked)
        }, POLL_DELAY_MS)
    }

    private fun complete(paused: Boolean, source: WebChatRealtimeVoiceBackgroundControlSource) {
        val detail = when {
            paused && source == WebChatRealtimeVoiceBackgroundControlSource.MEDIA ->
                "检测到其他媒体播放，实时语音已自动暂停"
            paused -> "实时语音已暂停"
            else -> "实时语音已继续"
        }
        onCompleted(paused, detail)
    }

    private fun fail(detail: String) {
        onFailed(detail)
    }

    private companion object {
        const val MAX_ATTEMPTS = 12
        const val POLL_DELAY_MS = 250L
    }
}
