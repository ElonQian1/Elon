package com.elon.app.chatgptweb

import android.webkit.WebView
import com.elon.app.beginWebChatBackgroundInteraction
import com.elon.app.showWebChatBackgroundSurface

internal class ChatGptRealtimeVoiceBackingController(
    private val ensureInitialized: () -> Unit,
    private val webView: () -> WebView?,
    private val surfaceMode: ChatGptWebSurfaceModeController,
    private val requestExecution: () -> Unit,
    private val schedule: (Runnable, Long) -> Unit,
    private val conversationRecovered: () -> Boolean,
) {
    private var active = false

    fun isActive(): Boolean = active

    fun begin(): Boolean {
        ensureInitialized()
        val view = webView() ?: return false
        active = true
        surfaceMode.select(ChatGptWebPresentationMode.NATIVE)
        view.beginWebChatBackgroundInteraction()
        requestExecution()
        return true
    }

    fun restoreAfterHostResume() {
        if (active) webView()?.beginWebChatBackgroundInteraction()
    }

    fun end(gracefulExit: Boolean) {
        if (!active) return
        active = false
        val view = webView() ?: return
        view.showWebChatBackgroundSurface()
        if (!gracefulExit) view.reload()
        requestExecution()
        if (gracefulExit) schedule(Runnable {
            if (active || conversationRecovered()) return@Runnable
            webView()?.reload()
            requestExecution()
        }, RECOVERY_TIMEOUT_MS)
    }

    fun release() {
        active = false
    }

    private companion object {
        const val RECOVERY_TIMEOUT_MS = 2_500L
    }
}
