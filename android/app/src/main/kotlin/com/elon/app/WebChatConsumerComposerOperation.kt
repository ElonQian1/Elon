package com.elon.app

internal data class WebChatConsumerComposerFeedback(
    val providerId: WebChatProviderId,
    val message: String,
)

internal object WebChatConsumerComposerOperationPolicy {
    fun resolve(
        provider: WebChatProviderIdentity,
        attachmentPhase: String,
        feedback: WebChatConsumerComposerFeedback?,
    ): WebChatConsumerRecoveryState = when (attachmentPhase) {
        "uploading", "sending" -> status("附件上传中，完成后会自动发送")
        "failed" -> status("附件发送失败，附件已保留，可重新发送")
        else -> feedback
            ?.takeIf { it.providerId == provider.id }
            ?.let { status(it.message) }
            ?: hidden()
    }

    fun commandAccepted(
        provider: WebChatProviderIdentity,
        action: String,
    ): WebChatConsumerComposerFeedback? {
        val message = when (action) {
            "chatgpt_start_dictation" -> "网页听写已开始，再点“工具”可完成"
            "chatgpt_submit_dictation" -> "网页听写已提交"
            "chatgpt_stop_generation" -> "已停止生成"
            "chatgpt_start_realtime_voice" -> "正在进入网页实时语音"
            else -> return null
        }
        return WebChatConsumerComposerFeedback(provider.id, message)
    }

    fun toolAccepted(
        provider: WebChatProviderIdentity,
        label: String,
    ): WebChatConsumerComposerFeedback = WebChatConsumerComposerFeedback(
        providerId = provider.id,
        message = "已切换网页工具：$label",
    )

    private fun status(message: String) = WebChatConsumerRecoveryState(
        visible = true,
        message = message,
        retryVisible = false,
        officialVisible = false,
    )

    private fun hidden() = WebChatConsumerRecoveryState(
        visible = false,
        message = "",
        retryVisible = false,
        officialVisible = false,
    )
}
