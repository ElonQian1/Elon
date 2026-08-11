package com.elon.app.chatgptweb

import android.content.Context
import java.net.URI

internal class ChatGptWebSessionStateStore(context: Context) {
    private val preferences = context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE)

    fun restoreUrl(): String = normalizeRestorableUrl(preferences.getString(KEY_URL, null))
        ?: ChatGptWebNavigationPolicy.START_URL

    fun saveUrl(rawUrl: String) {
        val normalized = normalizeRestorableUrl(rawUrl) ?: return
        preferences.edit().putString(KEY_URL, normalized).apply()
    }

    fun restoreMode(): ChatGptWebModeController.Mode? = preferences.getString(KEY_MODE, null)
        ?.let { value ->
            ChatGptWebModeController.Mode.values().firstOrNull { it.name == value }
        }

    fun saveMode(mode: ChatGptWebModeController.Mode) {
        preferences.edit().putString(KEY_MODE, mode.name).apply()
    }

    fun clear() {
        preferences.edit().clear().apply()
    }

    companion object {
        private const val PREFERENCES = "chatgpt_web_session_state"
        private const val KEY_URL = "last_safe_url"
        private const val KEY_MODE = "last_mode"
        private const val MAX_PATH_LENGTH = 512
        private val SAFE_PATH = Regex("/[A-Za-z0-9_./%~-]{0,$MAX_PATH_LENGTH}")
        private val RESTORABLE_PREFIXES = listOf(
            "/c/",
            "/g/",
            "/gpts",
            "/projects",
            "/tasks",
            "/library",
            "/apps",
            "/settings",
        )

        internal fun normalizeRestorableUrl(rawUrl: String?): String? {
            val uri = rawUrl?.let { runCatching { URI(it) }.getOrNull() } ?: return null
            if (!uri.scheme.equals("https", ignoreCase = true)) return null
            if (!uri.host.equals("chatgpt.com", ignoreCase = true)) return null
            if (uri.userInfo != null || (uri.port != -1 && uri.port != 443)) return null
            val path = uri.rawPath?.ifBlank { "/" } ?: "/"
            if (!SAFE_PATH.matches(path) || path.split('/').any { it == ".." }) return null
            val lowerPath = path.lowercase()
            if ("%2e" in lowerPath || "%2f" in lowerPath || "%5c" in lowerPath) return null
            if (path != "/" && RESTORABLE_PREFIXES.none { matchesPrefix(path, it) }) return null
            return "https://chatgpt.com$path"
        }

        private fun matchesPrefix(path: String, prefix: String): Boolean =
            if (prefix.endsWith('/')) path.startsWith(prefix)
            else path == prefix || path.startsWith("$prefix/")
    }
}
