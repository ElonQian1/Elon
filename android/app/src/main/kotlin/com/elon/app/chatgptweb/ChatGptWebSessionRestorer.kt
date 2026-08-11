package com.elon.app.chatgptweb

import android.content.Context

internal class ChatGptWebSessionRestorer(context: Context) {
    private val stateStore = ChatGptWebSessionStateStore(context)
    private var pendingMode = stateStore.restoreMode()

    fun restoreUrl(): String = stateStore.restoreUrl()

    fun onPageReady(url: String) {
        stateStore.saveUrl(url)
    }

    fun onModeShown(mode: ChatGptWebModeController.Mode) {
        if (pendingMode == null) stateStore.saveMode(mode)
    }

    fun restorePreferredMode(
        nativeModeEnabled: Boolean,
        controller: ChatGptWebModeController,
    ): Boolean {
        val target = pendingMode ?: return false
        if (target == ChatGptWebModeController.Mode.NATIVE && !nativeModeEnabled) return false
        pendingMode = null
        controller.select(target)
        return true
    }

    fun clear() {
        pendingMode = null
        stateStore.clear()
    }
}
