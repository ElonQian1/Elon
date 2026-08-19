package com.elon.app.chatgptweb

import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.webkit.WebView

internal class ChatGptNewConversationRecoveryCoordinator(
    private val webView: () -> WebView?,
    private val navigationActive: () -> Boolean,
    private val loading: () -> Boolean,
    private val composerReady: () -> Boolean,
    private val interactionRequested: () -> Unit,
    private val handler: Handler = Handler(Looper.getMainLooper()),
) {
    fun schedule() {
        cancel()
        handler.postDelayed(::recoverIfNeeded, RECOVERY_DELAY_MS)
    }

    fun cancel() {
        handler.removeCallbacksAndMessages(null)
    }

    private fun recoverIfNeeded() {
        val view = webView() ?: return
        val atHome = runCatching {
            val uri = Uri.parse(view.url.orEmpty())
            uri.scheme.equals("https", ignoreCase = true) &&
                uri.host.equals("chatgpt.com", ignoreCase = true) &&
                uri.path == "/"
        }.getOrDefault(false)
        when (ChatGptNewConversationRecoveryPolicy.action(
            navigationActive = navigationActive(),
            loading = loading(),
            composerReady = composerReady(),
            webViewAtHome = atHome,
        )) {
            ChatGptNewConversationRecoveryAction.NONE -> Unit
            ChatGptNewConversationRecoveryAction.RELOAD_HOME -> reload(view)
            ChatGptNewConversationRecoveryAction.LOAD_HOME -> loadHome(view)
        }
    }

    private fun reload(view: WebView) {
        interactionRequested()
        view.stopLoading()
        view.reload()
    }

    private fun loadHome(view: WebView) {
        interactionRequested()
        view.stopLoading()
        view.loadUrl(ChatGptWebNavigationPolicy.START_URL)
    }

    private companion object {
        const val RECOVERY_DELAY_MS = 3_000L
    }
}
