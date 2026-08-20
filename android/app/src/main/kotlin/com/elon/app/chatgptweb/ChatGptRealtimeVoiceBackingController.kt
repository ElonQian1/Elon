package com.elon.app.chatgptweb

import android.webkit.WebView
import com.elon.app.beginWebChatBackgroundInteraction
import com.elon.app.showWebChatBackgroundSurface

internal class ChatGptRealtimeVoiceBackingController(
    private val ensureInitialized: () -> Unit,
    private val webView: () -> WebView?,
    private val surfaceMode: ChatGptWebSurfaceModeController,
    private val requestExecution: () -> Unit,
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

    fun end() {
        if (!active) return
        active = false
        val view = webView() ?: return
        view.showWebChatBackgroundSurface()
        view.stopLoading()
        view.reload()
        requestExecution()
    }

    fun release() {
        active = false
    }
}
