package com.elon.app.chatgptweb

import java.net.URI
import java.net.URLDecoder
import java.net.URLEncoder
import java.nio.charset.StandardCharsets
import java.util.Locale

internal object ChatGptGoogleLoginHintPolicy {
    private const val GOOGLE_ACCOUNTS_HOST = "accounts.google.com"
    private const val LOGIN_HINT_PARAMETER = "login_hint"
    private const val MAX_ACCOUNT_NAME_LENGTH = 254
    private val authorizationPaths = setOf(
        "/o/oauth2/auth",
        "/o/oauth2/v2/auth",
    )

    fun normalizeAccountName(rawAccountName: String?): String? {
        val accountName = rawAccountName?.trim().orEmpty()
        if (accountName.length !in 3..MAX_ACCOUNT_NAME_LENGTH) return null
        if (accountName.any(Char::isWhitespace)) return null
        val separator = accountName.indexOf('@')
        if (separator <= 0 || separator != accountName.lastIndexOf('@')) return null
        if (separator == accountName.lastIndex) return null
        return accountName
    }

    fun rewriteAuthorizationUrl(rawUrl: String, rawAccountName: String?): String? {
        val accountName = normalizeAccountName(rawAccountName) ?: return null
        val uri = runCatching { URI(rawUrl) }.getOrNull() ?: return null
        if (!uri.scheme.equals("https", ignoreCase = true)) return null
        if (uri.host?.lowercase(Locale.ROOT) != GOOGLE_ACCOUNTS_HOST) return null
        if (uri.userInfo != null || (uri.port != -1 && uri.port != 443)) return null
        if (uri.path !in authorizationPaths) return null
        if (hasLoginHint(uri.rawQuery)) return null

        val fragmentIndex = rawUrl.indexOf('#')
        val baseUrl = if (fragmentIndex >= 0) rawUrl.substring(0, fragmentIndex) else rawUrl
        val fragment = if (fragmentIndex >= 0) rawUrl.substring(fragmentIndex) else ""
        val separator = when {
            '?' !in baseUrl -> "?"
            baseUrl.endsWith('?') || baseUrl.endsWith('&') -> ""
            else -> "&"
        }
        val encodedAccountName = URLEncoder.encode(
            accountName,
            StandardCharsets.UTF_8.name(),
        ).replace("+", "%20")
        return "$baseUrl$separator$LOGIN_HINT_PARAMETER=$encodedAccountName$fragment"
    }

    private fun hasLoginHint(rawQuery: String?): Boolean = rawQuery
        ?.split('&')
        ?.any { parameter ->
            val rawName = parameter.substringBefore('=')
            runCatching {
                URLDecoder.decode(rawName, StandardCharsets.UTF_8.name())
            }.getOrNull() == LOGIN_HINT_PARAMETER
        }
        ?: false
}
