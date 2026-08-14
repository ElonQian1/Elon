package com.elon.app.googleweb

import java.net.URI

internal object GoogleWebNavigationPolicy {
    const val START_URL = "https://www.google.com/aimode"

    fun allows(rawUrl: String?): Boolean {
        val uri = parse(rawUrl) ?: return false
        if (uri.scheme != "https" || uri.userInfo != null || uri.port !in setOf(-1, 443)) return false
        return uri.host?.lowercase() in ALLOWED_HOSTS
    }

    fun supportsAiMode(rawUrl: String?): Boolean {
        val uri = parse(rawUrl) ?: return false
        if (!allows(rawUrl)) return false
        return when (uri.path) {
            "/aimode" -> true
            "/webhp" -> queryValue(uri.rawQuery, "aep") == "11"
            "/search" -> queryValue(uri.rawQuery, "udm") == "50" ||
                queryValue(uri.rawQuery, "aep") == "11"
            else -> false
        }
    }

    fun sanitizeRestorableUrl(rawUrl: String?): String? {
        val bounded = rawUrl?.take(MAX_URL_LENGTH) ?: return null
        val uri = parse(bounded)?.takeIf { supportsAiMode(bounded) } ?: return null
        val query = canonicalQuery(uri.path, uri.rawQuery)
        return buildString {
            append("https://www.google.com")
            append(uri.path)
            if (query.isNotEmpty()) append('?').append(query)
        }.take(MAX_URL_LENGTH)
    }

    fun displayHost(rawUrl: String?): String = parse(rawUrl)?.host ?: "google.com"

    private fun parse(rawUrl: String?): URI? = runCatching { URI(rawUrl.orEmpty()) }.getOrNull()

    private fun queryValue(rawQuery: String?, name: String): String? = rawQuery
        ?.split('&')
        ?.firstOrNull { it.substringBefore('=') == name }
        ?.substringAfter('=', "")

    private fun canonicalQuery(path: String, rawQuery: String?): String {
        if (path == "/aimode") return ""
        val values = rawQuery.orEmpty().split('&')
            .filter(String::isNotBlank)
            .associateBy { it.substringBefore('=') }
        val keys = when (path) {
            "/webhp" -> listOf("aep", "q", "hl")
            "/search" -> listOf("q", "udm", "aep", "hl")
            else -> emptyList()
        }
        return keys.mapNotNull(values::get).joinToString("&")
    }

    private val ALLOWED_HOSTS = setOf("google.com", "www.google.com")
    private const val MAX_URL_LENGTH = 8_192
}
