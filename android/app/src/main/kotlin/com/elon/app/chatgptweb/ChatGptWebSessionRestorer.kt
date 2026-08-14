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
        val decision = ChatGptWebModeRestorePolicy.decide(
            pending = pendingMode,
            current = controller.selectedMode(),
            nativeModeEnabled = nativeModeEnabled,
        )
        val target = decision.target ?: return false
        if (decision.consumePending) pendingMode = null
        controller.select(target)
        return true
    }

    fun clear() {
        pendingMode = null
        stateStore.clear()
    }
}
