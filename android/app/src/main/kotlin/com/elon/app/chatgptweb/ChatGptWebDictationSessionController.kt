package com.elon.app.chatgptweb

internal class ChatGptWebDictationSessionController(
    private val isNativeSelected: () -> Boolean,
    private val showOfficial: () -> Unit,
    private val restoreNative: () -> Unit,
    private val cancelOfficial: () -> Unit,
) {
    private var active = false
    private var restoreNativeAfterSession = false

    fun onSnapshot(dictationActive: Boolean) {
        if (dictationActive == active) return

        active = dictationActive
        if (dictationActive) {
            restoreNativeAfterSession = isNativeSelected()
            showOfficial()
        } else {
            if (restoreNativeAfterSession) restoreNative()
            restoreNativeAfterSession = false
        }
    }

    fun handleBack(): Boolean {
        if (!active) return false
        cancelOfficial()
        return true
    }

    fun reset() {
        active = false
        restoreNativeAfterSession = false
    }
}
