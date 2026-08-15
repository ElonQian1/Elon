package com.elon.app

import androidx.annotation.DrawableRes

internal enum class WebChatProviderId(val wireValue: String) {
    CHATGPT_WEB("chatgpt_web"),
    GOOGLE_WEB("google_web");

    companion object {
        fun fromWireValue(value: String?): WebChatProviderId =
            entries.firstOrNull { it.wireValue == value } ?: CHATGPT_WEB
    }
}

internal enum class WebChatProviderCapability {
    CONVERSATION_LIST,
    PROJECT_LIST,
    NEW_CONVERSATION,
    MESSAGE_COPY,
    MESSAGE_REGENERATE,
    MESSAGE_CONTEXT_ACTIONS,
    MODEL_SELECTOR,
    ATTACHMENT_UPLOAD,
    COMPOSER_TOOLS,
    STOP_GENERATION,
    DICTATION,
    REALTIME_VOICE,
    RICH_TEXT,
    RICH_PARTS,
}

internal data class WebChatProviderIdentity(
    val id: WebChatProviderId,
    val displayName: String,
    @DrawableRes val avatarResId: Int,
    val available: Boolean,
    val capabilities: Set<WebChatProviderCapability>,
) {
    fun supports(capability: WebChatProviderCapability): Boolean = capability in capabilities

    val selectable: Boolean
        get() = available && REQUIRED_NATIVE_NAVIGATION.all(capabilities::contains)

    companion object {
        val REQUIRED_NATIVE_NAVIGATION = setOf(
            WebChatProviderCapability.CONVERSATION_LIST,
            WebChatProviderCapability.PROJECT_LIST,
            WebChatProviderCapability.NEW_CONVERSATION,
        )
    }
}

internal object WebChatProviderRegistry {
    private val providers = listOf(
        WebChatProviderIdentity(
            id = WebChatProviderId.CHATGPT_WEB,
            displayName = "ChatGPT 网页 AI",
            avatarResId = R.drawable.ic_web_ai_chatgpt_avatar,
            available = true,
            capabilities = WebChatProviderIdentity.REQUIRED_NATIVE_NAVIGATION + setOf(
                WebChatProviderCapability.MESSAGE_COPY,
                WebChatProviderCapability.MESSAGE_REGENERATE,
                WebChatProviderCapability.MESSAGE_CONTEXT_ACTIONS,
                WebChatProviderCapability.MODEL_SELECTOR,
                WebChatProviderCapability.ATTACHMENT_UPLOAD,
                WebChatProviderCapability.COMPOSER_TOOLS,
                WebChatProviderCapability.STOP_GENERATION,
                WebChatProviderCapability.DICTATION,
                WebChatProviderCapability.REALTIME_VOICE,
                WebChatProviderCapability.RICH_TEXT,
                WebChatProviderCapability.RICH_PARTS,
            ),
        ),
        WebChatProviderIdentity(
            id = WebChatProviderId.GOOGLE_WEB,
            displayName = "Google 搜索网页 AI",
            avatarResId = R.drawable.ic_web_ai_google_placeholder_avatar,
            available = true,
            capabilities = WebChatProviderIdentity.REQUIRED_NATIVE_NAVIGATION + setOf(
                WebChatProviderCapability.MESSAGE_COPY,
                WebChatProviderCapability.RICH_TEXT,
            ),
        ),
    )

    fun get(id: WebChatProviderId): WebChatProviderIdentity =
        providers.first { it.id == id }

    fun available(): List<WebChatProviderIdentity> = providers.filter(WebChatProviderIdentity::selectable)
}
