package com.elon.app.chatgptweb

import java.net.URI

internal object ChatGptWebConversationPath {
    private val SAFE_PATH = Regex(
        "(?:/c/[A-Za-z0-9_-]{1,160}|/g/(g-p-[A-Za-z0-9_-]{1,160})/c/[A-Za-z0-9_-]{1,160})",
    )
    private val PROJECT_PATH = Regex("/g/(g-p-[A-Za-z0-9_-]{1,160})(?:/project)?")
    private val PROJECT_ID = Regex("g-p-[A-Za-z0-9_-]{1,160}")
    private val PRODUCTION_PROJECT_ID = Regex("(g-p-[A-Fa-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?")

    fun normalize(path: String?): String? = path?.trim()?.takeIf(SAFE_PATH::matches)

    fun normalizeProject(path: String?): String? = path?.trim()
        ?.let(PROJECT_PATH::matchEntire)
        ?.groupValues
        ?.getOrNull(1)
        ?.let(::canonicalProjectId)
        ?.let { "/g/$it/project" }

    fun identity(path: String?): String? = normalize(path)
        ?.substringAfterLast('/')
        ?.takeIf(String::isNotBlank)

    fun projectId(path: String?): String? = path?.trim()?.let { value ->
        val raw = SAFE_PATH.matchEntire(value)?.groupValues?.getOrNull(1)?.takeIf(String::isNotBlank)
            ?: PROJECT_PATH.matchEntire(value)?.groupValues?.getOrNull(1)?.takeIf(String::isNotBlank)
        canonicalProjectId(raw)
    }

    fun canonicalProjectId(value: String?): String? {
        val id = value?.trim()?.takeIf(PROJECT_ID::matches) ?: return null
        return PRODUCTION_PROJECT_ID.matchEntire(id)?.groupValues?.getOrNull(1) ?: id
    }

    fun fromUrl(url: String?): String? {
        val uri = runCatching { URI(url.orEmpty()) }.getOrNull() ?: return null
        if (!uri.scheme.equals("https", ignoreCase = true)) return null
        if (!uri.host.equals("chatgpt.com", ignoreCase = true)) return null
        if (uri.userInfo != null || (uri.port != -1 && uri.port != 443)) return null
        return normalize(uri.path)
    }
}
