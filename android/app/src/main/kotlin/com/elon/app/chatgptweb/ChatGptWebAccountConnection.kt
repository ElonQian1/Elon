package com.elon.app.chatgptweb

import android.content.Context
import android.content.SharedPreferences
import java.net.URI

/** A presentation hint for the existing local WebView profile, never a send permission. */
internal class ChatGptWebAccountConnection(context: Context) {
    enum class State { UNKNOWN, CONNECTED, LOGIN_REQUIRED }

    private val preferences = context.applicationContext.getSharedPreferences("chatgpt_account_connection_v1", 0)

    fun state(): State = State.entries.firstOrNull { it.name == preferences.getString("state", null) }
        ?: State.UNKNOWN

    fun observe(event: ChatGptWebEvent) {
        val snapshot = (event as? ChatGptWebEvent.Snapshot)?.value ?: return
        val next = observedState(snapshot) ?: return
        if (next != state()) preferences.edit().putString("state", next.name).apply()
    }

    fun subscribe(changed: () -> Unit): () -> Unit {
        val listener = SharedPreferences.OnSharedPreferenceChangeListener { _, key ->
            if (key == "state") changed()
        }
        preferences.registerOnSharedPreferenceChangeListener(listener)
        return { preferences.unregisterOnSharedPreferenceChangeListener(listener) }
    }

    companion object {
        fun observedState(snapshot: ChatGptWebSnapshot): State? {
            if (snapshot.contentOnly) return null
            val uri = runCatching { URI(snapshot.url) }.getOrNull() ?: return null
            if (uri.scheme != "https" || uri.host != "chatgpt.com" || uri.userInfo != null ||
                uri.port !in listOf(-1, 443)) return null
            if (snapshot.authenticated && snapshot.accountConfirmed && !ChatGptWebAccessPolicy.requiresLogin(snapshot) &&
                !ChatGptWebNavigationPolicy.isAuthenticationPage(snapshot.url)) return State.CONNECTED
            // Missing DOM, a challenge page or an anonymous-looking loading frame is not logout.
            if (!snapshot.authenticated && snapshot.loginRequired && snapshot.pageKind == "auth") {
                return State.LOGIN_REQUIRED
            }
            return null
        }
    }
}
