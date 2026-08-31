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
        dictationActive: Boolean = false,
        imageGenerationActive: Boolean = false,
        streaming: Boolean = false,
        imagePreviewState: String = "idle",
    ): WebChatConsumerRecoveryState = when (attachmentPhase) {
        "uploading", "sending" -> status("附件上传中，完成后会自动发送")
        "failed" -> status("附件发送失败，附件已保留，可重新发送")
        else -> when {
            imageGenerationActive && streaming -> status("正在创建图片…")
            imageGenerationActive && imagePreviewState == "preparing" ->
                status("图片已生成，正在准备预览…")
            imageGenerationActive && imagePreviewState == "failed" ->
                status("图片已生成，预览同步失败，可在“图像”中重试")
            feedback?.providerId == provider.id -> status(feedback.message)
            dictationActive -> status("正在听写，点蓝色勾完成，点×取消")
            else -> hidden()
        }
    }

    fun commandAccepted(
        provider: WebChatProviderIdentity,
        action: String,
    ): WebChatConsumerComposerFeedback? {
        val message = when (action) {
            "chatgpt_start_dictation" -> "正在听写，点蓝色勾完成，点×取消"
            "chatgpt_submit_dictation" -> "正在完成网页听写"
            "chatgpt_cancel_dictation" -> "正在取消网页听写"
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
