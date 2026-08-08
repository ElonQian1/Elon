package com.elon.app.chatgptweb

import java.net.URI
import java.util.Locale

internal object ChatGptWebNavigationPolicy {
    const val START_URL = "https://chatgpt.com/"
    const val AUTH_URL = "https://chatgpt.com/auth/login"

    private val allowedDomainSuffixes = setOf(
        "chatgpt.com",
        "openai.com",
    )

    private val allowedIdentityHosts = setOf(
        "accounts.google.com",
        "appleid.apple.com",
        "login.live.com",
        "login.microsoftonline.com",
    )

    fun allows(rawUrl: String): Boolean {
        val uri = parse(rawUrl) ?: return false
        if (!uri.scheme.equals("https", ignoreCase = true)) return false
        if (uri.userInfo != null || (uri.port != -1 && uri.port != 443)) return false

        val host = uri.host?.lowercase(Locale.ROOT) ?: return false
        return host in allowedIdentityHosts || allowedDomainSuffixes.any { domain ->
            host == domain || host.endsWith(".$domain")
        }
    }

    fun displayHost(rawUrl: String?): String = rawUrl
        ?.let(::parse)
        ?.host
        ?.lowercase(Locale.ROOT)
        ?: "chatgpt.com"

    fun supportsEnhancedMode(rawUrl: String?): Boolean {
        val uri = rawUrl?.let(::parse) ?: return false
        return uri.scheme.equals("https", ignoreCase = true) &&
            uri.host.equals("chatgpt.com", ignoreCase = true) &&
            uri.userInfo == null &&
            (uri.port == -1 || uri.port == 443)
    }

    fun isAuthenticationPage(rawUrl: String?): Boolean {
        val uri = rawUrl?.let(::parse) ?: return false
        return supportsEnhancedMode(rawUrl) && uri.path.startsWith("/auth")
    }

    private fun parse(rawUrl: String): URI? = runCatching { URI(rawUrl) }.getOrNull()
}
