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
internal data class WebChatProviderIdentity(
    val id: WebChatProviderId,
    val displayName: String,
    @DrawableRes val avatarResId: Int,
    val available: Boolean,
)

internal object WebChatProviderRegistry {
    private val providers = listOf(
        WebChatProviderIdentity(
            id = WebChatProviderId.CHATGPT_WEB,
            displayName = "ChatGPT 网页 AI",
            avatarResId = R.drawable.ic_web_ai_chatgpt_avatar,
            available = true,
        ),
        WebChatProviderIdentity(
            id = WebChatProviderId.GOOGLE_WEB,
            displayName = "Google 搜索网页 AI",
            avatarResId = R.drawable.ic_web_ai_google_placeholder_avatar,
            available = false,
        ),
    )

    fun get(id: WebChatProviderId): WebChatProviderIdentity =
        providers.first { it.id == id }

    fun available(): List<WebChatProviderIdentity> = providers.filter(WebChatProviderIdentity::available)
}
