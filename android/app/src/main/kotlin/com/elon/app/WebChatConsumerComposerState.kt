package com.elon.app

internal data class WebChatConsumerComposerState(
    val attachmentVisible: Boolean,
    val toolsVisible: Boolean,
    val inputHint: String,
)

internal object WebChatConsumerComposerStateResolver {
    fun resolve(
        provider: WebChatProviderIdentity,
        state: String,
        composerReady: Boolean,
        attachmentSupported: Boolean,
    ): WebChatConsumerComposerState = WebChatConsumerComposerState(
        attachmentVisible = composerReady &&
            attachmentSupported &&
            provider.supports(WebChatProviderCapability.ATTACHMENT_UPLOAD),
        toolsVisible = composerReady && provider.supports(WebChatProviderCapability.COMPOSER_TOOLS),
        inputHint = when {
            state == "error" -> "网页连接异常，输入内容将保留"
            state == "login_required" -> "当前网页要求登录，输入内容将保留"
            !composerReady -> "正在连接${provider.displayName}…"
            else -> "输入内容"
        },
    )
}
