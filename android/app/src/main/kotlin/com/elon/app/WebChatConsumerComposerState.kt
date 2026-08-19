package com.elon.app

internal data class WebChatConsumerComposerState(
    val attachmentVisible: Boolean,
    val toolsVisible: Boolean,
    val submissionEnabled: Boolean,
    val inputHint: String,
)

internal object WebChatConsumerComposerStateResolver {
    fun resolve(
        provider: WebChatProviderIdentity,
        state: String,
        composerReady: Boolean,
        attachmentSupported: Boolean,
        warmSessionAvailable: Boolean = false,
    ): WebChatConsumerComposerState {
        val submissionEnabled = state == "ready" && composerReady
        val retainWarmPresentation = state == "loading" && warmSessionAvailable
        val controlsVisible = submissionEnabled || retainWarmPresentation
        return WebChatConsumerComposerState(
            attachmentVisible = controlsVisible &&
                attachmentSupported &&
                provider.supports(WebChatProviderCapability.ATTACHMENT_UPLOAD),
            toolsVisible = controlsVisible &&
                provider.supports(WebChatProviderCapability.COMPOSER_TOOLS),
            submissionEnabled = submissionEnabled,
            inputHint = when {
                state == "error" -> "网页连接异常，输入内容将保留"
                state == "login_required" -> "当前网页要求登录，输入内容将保留"
                retainWarmPresentation -> "输入内容"
                !composerReady -> "正在连接${provider.displayName}…"
                else -> "输入内容"
            },
        )
    }
}
