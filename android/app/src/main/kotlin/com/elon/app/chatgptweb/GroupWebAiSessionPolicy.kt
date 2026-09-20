package com.elon.app.chatgptweb

import com.elon.app.WebChatProviderId
import com.elon.app.googleweb.GoogleWebNavigationPolicy
import java.net.URI

internal object GroupWebAiSessionPolicy {
    fun startUrl(provider: WebChatProviderId): String = when (provider) {
        WebChatProviderId.CHATGPT_WEB -> "https://chatgpt.com/?temporary-chat=true"
        WebChatProviderId.GOOGLE_WEB -> GoogleWebNavigationPolicy.START_URL
    }

    fun ready(snapshot: ChatGptWebSnapshot, provider: WebChatProviderId, documentUrl: String = snapshot.url): Boolean {
        if (snapshot.loginRequired || snapshot.streaming || snapshot.messages.isNotEmpty() || snapshot.draft.isNotBlank()) return false
        val uri = runCatching { URI(documentUrl) }.getOrNull() ?: return false
        val observed = runCatching { URI(snapshot.url) }.getOrNull() ?: return false
        // Snapshots intentionally omit query parameters. Read route scope from this WebView,
        // but never combine an old conversation snapshot with a different live document.
        if (listOf(uri, observed).any { it.scheme != "https" || it.userInfo != null ||
                it.port !in listOf(-1, 443) || it.fragment != null } ||
            uri.host != observed.host || uri.path.orEmpty().ifEmpty { "/" } != observed.path.orEmpty().ifEmpty { "/" }) return false
        val query = uri.rawQuery.orEmpty().split('&').filter(String::isNotBlank)
        return when (provider) {
            WebChatProviderId.CHATGPT_WEB -> (snapshot.composerReady || snapshot.privateSendReady) &&
                uri.host == "chatgpt.com" && uri.path in listOf("", "/") && query == listOf("temporary-chat=true")
            WebChatProviderId.GOOGLE_WEB -> snapshot.composerReady &&
                GoogleWebNavigationPolicy.supportsAiMode(documentUrl) &&
                query.all { it.substringBefore('=') in setOf("udm", "aep", "hl", "authuser") }
        }
    }
}
