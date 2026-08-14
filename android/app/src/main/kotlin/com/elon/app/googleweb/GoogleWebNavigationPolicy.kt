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
            "/search" -> queryValue(uri.rawQuery, "udm") == "50" ||
                queryValue(uri.rawQuery, "aep") == "11"
            else -> false
        }
    }

    fun sanitizeRestorableUrl(rawUrl: String?): String? = rawUrl
        ?.take(MAX_URL_LENGTH)
        ?.takeIf(::supportsAiMode)

    fun displayHost(rawUrl: String?): String = parse(rawUrl)?.host ?: "google.com"

    private fun parse(rawUrl: String?): URI? = runCatching { URI(rawUrl.orEmpty()) }.getOrNull()

    private fun queryValue(rawQuery: String?, name: String): String? = rawQuery
        ?.split('&')
        ?.firstOrNull { it.substringBefore('=') == name }
        ?.substringAfter('=', "")

    private val ALLOWED_HOSTS = setOf("google.com", "www.google.com")
    private const val MAX_URL_LENGTH = 8_192
}
