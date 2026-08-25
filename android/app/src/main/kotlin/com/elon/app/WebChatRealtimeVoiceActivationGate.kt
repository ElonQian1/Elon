package com.elon.app

internal data class WebChatRealtimeVoiceActivationEvidence(
    val androidPermissionGranted: Boolean?,
    val webPermissionGrantRevision: Long,
    val webRequestPending: Boolean,
    val requestState: String,
    val officialVoiceActive: Boolean = false,
)

internal sealed interface WebChatRealtimeVoiceActivationDecision {
    data class Wait(val detail: String) : WebChatRealtimeVoiceActivationDecision
    data object Active : WebChatRealtimeVoiceActivationDecision
    data class Rejected(val detail: String) : WebChatRealtimeVoiceActivationDecision
    data object Unconfirmed : WebChatRealtimeVoiceActivationDecision
}

internal object WebChatRealtimeVoiceActivationEvidencePolicy {
    fun resolve(
        permission: WebChatRealtimeVoiceActivationEvidence,
        state: WebChatConsumerState?,
    ): WebChatRealtimeVoiceActivationEvidence = permission.copy(
        officialVoiceActive = state?.adapterCurrent == true &&
            WebChatRealtimeVoiceEndPolicy.resolve(state.controls) != null,
    )
}

internal class WebChatRealtimeVoiceActivationGate(
    private val maxPolls: Int = 40,
) {
    private var grantRevisionBeforeStart = 0L

    fun begin(evidence: WebChatRealtimeVoiceActivationEvidence) {
        grantRevisionBeforeStart = evidence.webPermissionGrantRevision
    }

    fun observe(
        evidence: WebChatRealtimeVoiceActivationEvidence,
        attempt: Int,
        pollLimit: Int = maxPolls,
    ): WebChatRealtimeVoiceActivationDecision {
        if (evidence.androidPermissionGranted == false || evidence.requestState in DENIED_STATES) {
            return WebChatRealtimeVoiceActivationDecision.Rejected(
                "麦克风权限未开启，请授权后重试",
            )
        }
        if (evidence.officialVoiceActive) {
            return WebChatRealtimeVoiceActivationDecision.Active
        }
        if (evidence.webPermissionGrantRevision > grantRevisionBeforeStart) {
            return WebChatRealtimeVoiceActivationDecision.Active
        }
        if (attempt >= pollLimit) {
            return WebChatRealtimeVoiceActivationDecision.Unconfirmed
        }
        val detail = if (evidence.webRequestPending) {
            "正在授权官网麦克风"
        } else {
            "正在等待官网启动麦克风"
        }
        return WebChatRealtimeVoiceActivationDecision.Wait(detail)
    }

    private companion object {
        val DENIED_STATES = setOf(
            "permission_denied",
            "web_request_rejected",
            "web_request_canceled",
            "disposed",
        )
    }
}
