package com.elon.app

internal enum class WebChatProductionObservationState {
    AVAILABLE,
    SESSION_RECOVERING,
    SYNCING,
    TEMPORARILY_UNOBSERVED,
    REQUEST_FAILED,
    ADAPTER_UNSUPPORTED,
}

internal data class WebChatProductionCapabilityEvidence(
    val declaredSupported: Boolean,
    val adapterCurrent: Boolean,
    val observedCount: Int,
    val cachedCount: Int = 0,
    val requestAccepted: Boolean? = null,
    val requestError: String? = null,
    val requestStatus: WebChatConsumerCommandStatus? = null,
    val pollingExhausted: Boolean = false,
)

/**
 * Separates product capability evidence from a transient DOM observation.
 * An empty observation can only mean syncing or temporarily unobserved.
 */
internal object WebChatProductionCapabilityEvidencePolicy {
    fun resolve(
        evidence: WebChatProductionCapabilityEvidence,
    ): WebChatProductionObservationState {
        require(evidence.observedCount >= 0)
        require(evidence.cachedCount >= 0)

        if (!evidence.declaredSupported || evidence.requestError in EXPLICIT_UNSUPPORTED_ERRORS) {
            return WebChatProductionObservationState.ADAPTER_UNSUPPORTED
        }
        if (evidence.observedCount > 0 || evidence.cachedCount > 0) {
            return WebChatProductionObservationState.AVAILABLE
        }
        if (!evidence.adapterCurrent || evidence.requestError in SESSION_RECOVERY_ERRORS) {
            return WebChatProductionObservationState.SESSION_RECOVERING
        }
        if (
            evidence.requestAccepted == false ||
            evidence.requestStatus == WebChatConsumerCommandStatus.FAILED ||
            evidence.requestStatus == WebChatConsumerCommandStatus.TIMED_OUT
        ) {
            return WebChatProductionObservationState.REQUEST_FAILED
        }
        return if (evidence.pollingExhausted) {
            WebChatProductionObservationState.TEMPORARILY_UNOBSERVED
        } else {
            WebChatProductionObservationState.SYNCING
        }
    }

    fun requestStatus(
        request: WebChatConsumerCommandResult?,
        state: WebChatConsumerState?,
    ): WebChatConsumerCommandStatus? {
        val requestId = request?.requestId ?: return null
        return state?.commandRequests?.firstOrNull { it.id == requestId }?.status
    }

    fun subtitle(state: WebChatProductionObservationState): String = when (state) {
        WebChatProductionObservationState.AVAILABLE -> "已同步"
        WebChatProductionObservationState.SESSION_RECOVERING -> "网页会话恢复后自动同步"
        WebChatProductionObservationState.SYNCING -> "后台同步中"
        WebChatProductionObservationState.TEMPORARILY_UNOBSERVED ->
            "本次暂未读取到，稍后可重试"
        WebChatProductionObservationState.REQUEST_FAILED -> "同步暂时失败，稍后可重试"
        WebChatProductionObservationState.ADAPTER_UNSUPPORTED -> "当前一龙原生适配尚未接入"
    }

    fun selectorState(state: WebChatProductionObservationState): String = when (state) {
        WebChatProductionObservationState.AVAILABLE -> "available"
        WebChatProductionObservationState.SESSION_RECOVERING -> "recovering"
        WebChatProductionObservationState.SYNCING -> "syncing"
        WebChatProductionObservationState.TEMPORARILY_UNOBSERVED -> "not-observed"
        WebChatProductionObservationState.REQUEST_FAILED -> "sync-failed"
        WebChatProductionObservationState.ADAPTER_UNSUPPORTED -> "adapter-unsupported"
    }

    private val SESSION_RECOVERY_ERRORS = setOf(
        "bridge_not_ready",
        "adapter_not_current",
        "snapshot_unavailable",
    )
    private val EXPLICIT_UNSUPPORTED_ERRORS = setOf(
        "unsupported_consumer_command",
        "adapter_unsupported",
        "platform_unsupported",
        "webview_unsupported",
    )
}
