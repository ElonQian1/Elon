package com.elon.app.chatgptweb

import com.elon.app.WebChatProviderId
import com.elon.app.googleweb.GoogleWebNavigationPolicy
import java.net.URI

internal object GroupWebAiSessionPolicy {
    fun startUrl(provider: WebChatProviderId): String = when (provider) {
        WebChatProviderId.CHATGPT_WEB -> "https://chatgpt.com/?temporary-chat=true"
        WebChatProviderId.GOOGLE_WEB -> GoogleWebNavigationPolicy.START_URL
    }

    fun ready(snapshot: ChatGptWebSnapshot, provider: WebChatProviderId): Boolean {
        if (snapshot.loginRequired || snapshot.streaming || snapshot.messages.isNotEmpty() || snapshot.draft.isNotBlank()) return false
        val uri = runCatching { URI(snapshot.url) }.getOrNull() ?: return false
        if (uri.scheme != "https" || uri.userInfo != null || uri.port !in listOf(-1, 443)) return false
        val query = uri.rawQuery.orEmpty().split('&').filter(String::isNotBlank)
        return when (provider) {
            WebChatProviderId.CHATGPT_WEB -> (snapshot.composerReady || snapshot.privateSendReady) &&
                uri.host == "chatgpt.com" && uri.path in listOf("", "/") && "temporary-chat=true" in query
            WebChatProviderId.GOOGLE_WEB -> snapshot.composerReady &&
                GoogleWebNavigationPolicy.supportsAiMode(snapshot.url) &&
                query.all { it.substringBefore('=') in setOf("udm", "aep", "hl", "authuser") }
        }
    }
}
