package com.elon.app

internal data class WebChatRealtimeVoiceActivationEvidence(
    val androidPermissionGranted: Boolean?,
    val webPermissionGrantRevision: Long,
    val webRequestPending: Boolean,
    val requestState: String,
)

internal sealed interface WebChatRealtimeVoiceActivationDecision {
    data class Wait(val detail: String) : WebChatRealtimeVoiceActivationDecision
    data object Active : WebChatRealtimeVoiceActivationDecision
    data class Failed(val detail: String) : WebChatRealtimeVoiceActivationDecision
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
            return WebChatRealtimeVoiceActivationDecision.Failed(
                "麦克风权限未开启，请授权后重试",
            )
        }
        if (evidence.webPermissionGrantRevision > grantRevisionBeforeStart) {
            return WebChatRealtimeVoiceActivationDecision.Active
        }
        if (attempt >= pollLimit) {
            return WebChatRealtimeVoiceActivationDecision.Failed(
                "没有检测到官网麦克风启动，可重试或打开官网语音",
            )
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
