package com.elon.app.chatgptweb

import java.net.URI
import java.net.URLDecoder

internal object WebChatSnapshotPrivacyPolicy {
    fun canPersist(providerKey: String, url: String): Boolean {
        if (providerKey != "chatgpt") return true
        return runCatching {
            val uri = URI(url)
            if (uri.scheme != "https" || uri.host != "chatgpt.com" ||
                uri.userInfo != null || uri.port !in setOf(-1, 443)
            ) return false

            val privacyValues = uri.rawQuery.orEmpty().split('&').mapNotNull { pair ->
                val parts = pair.split('=', limit = 2)
                val name = URLDecoder.decode(parts[0], "UTF-8")
                val value = URLDecoder.decode(parts.getOrElse(1) { "" }, "UTF-8")
                value.takeIf { name == "temporary-chat" }
            }
            // Unknown or conflicting privacy flags must not create a persistent snapshot.
            privacyValues.isEmpty() || privacyValues == listOf("false")
        }.getOrDefault(false)
    }
}
