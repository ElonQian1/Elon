package com.elon.app.chatgptweb

import android.webkit.WebView
import com.elon.app.showWebChatBackgroundSurface
import com.elon.app.showWebChatSkinSurface

internal class ChatGptWebSurfaceModeController(
    private val webView: () -> WebView?,
    private val pageAdapter: () -> ChatGptWebPageAdapter?,
    private val requestExecution: () -> Unit,
    private val ensureInitialized: () -> Unit,
) {
    private var mode = ChatGptWebPresentationMode.NATIVE

    fun mode(): ChatGptWebPresentationMode = mode

    fun isSkin(): Boolean = mode == ChatGptWebPresentationMode.SKIN

    fun select(next: ChatGptWebPresentationMode): Boolean {
        if (next != ChatGptWebPresentationMode.NATIVE && next != ChatGptWebPresentationMode.SKIN) {
            return false
        }
        mode = next
        if (isSkin()) ensureInitialized()
        apply()
        return true
    }

    fun apply() {
        val skinEnabled = isSkin()
        pageAdapter()?.setSkinMode(skinEnabled)
        webView()?.let { view ->
            if (skinEnabled) {
                requestExecution()
                view.showWebChatSkinSurface()
                view.requestFocus()
            } else {
                view.clearFocus()
                view.showWebChatBackgroundSurface()
            }
        }
    }

    fun onTouch() {
        if (isSkin()) requestExecution()
    }

    fun attach(view: WebView) {
        view.setOnTouchListener { _, _ ->
            onTouch()
            false
        }
    }
}
