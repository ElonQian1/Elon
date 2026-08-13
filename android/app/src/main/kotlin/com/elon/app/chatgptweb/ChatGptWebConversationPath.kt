package com.elon.app.chatgptweb

import java.net.URI

internal object ChatGptWebConversationPath {
    private val SAFE_PATH = Regex(
        "(?:/c/[A-Za-z0-9_-]{1,160}|/g/(g-p-[A-Za-z0-9_-]{1,160})/c/[A-Za-z0-9_-]{1,160})",
    )
    private val PROJECT_PATH = Regex("/g/(g-p-[A-Za-z0-9_-]{1,160})(?:/project)?")

    fun normalize(path: String?): String? = path?.trim()?.takeIf(SAFE_PATH::matches)

    fun normalizeProject(path: String?): String? = path?.trim()?.takeIf(PROJECT_PATH::matches)

    fun projectId(path: String?): String? = path?.trim()?.let { value ->
        SAFE_PATH.matchEntire(value)?.groupValues?.getOrNull(1)?.takeIf(String::isNotBlank)
            ?: PROJECT_PATH.matchEntire(value)?.groupValues?.getOrNull(1)?.takeIf(String::isNotBlank)
    }

    fun fromUrl(url: String?): String? {
        val uri = runCatching { URI(url.orEmpty()) }.getOrNull() ?: return null
        if (!uri.scheme.equals("https", ignoreCase = true)) return null
        if (!uri.host.equals("chatgpt.com", ignoreCase = true)) return null
        if (uri.userInfo != null || (uri.port != -1 && uri.port != 443)) return null
        return normalize(uri.path)
    }
}
