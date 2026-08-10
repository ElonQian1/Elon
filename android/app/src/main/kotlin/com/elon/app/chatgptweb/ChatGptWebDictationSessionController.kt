package com.elon.app.chatgptweb

internal class ChatGptWebDictationSessionController(
    private val isNativeSelected: () -> Boolean,
    private val showOfficial: () -> Unit,
    private val restoreNative: () -> Unit,
    private val cancelOfficial: () -> Unit,
) {
    private var active = false
    private var pending = false
    private var restoreNativeAfterSession = false
    private var startAttempt = 0L

    fun onStartRequested(): Long? {
        if (active || pending) return null
        startAttempt += 1
        pending = true
        restoreNativeAfterSession = isNativeSelected()
        showOfficial()
        return startAttempt
    }

    fun onSnapshot(dictationActive: Boolean) {
        if (dictationActive) {
            if (!active && !pending) restoreNativeAfterSession = isNativeSelected()
            active = true
            pending = false
            showOfficial()
            return
        }

        if (!active) return
        active = false
        finishSession()
    }

    fun onStartFailed() {
        if (!pending || active) return
        finishSession()
    }

    fun onStartTimedOut(attempt: Long) {
        if (attempt != startAttempt) return
        onStartFailed()
    }

    fun handleBack(): Boolean {
        return when {
            active -> {
                cancelOfficial()
                true
            }
            pending -> {
                finishSession()
                true
            }
            else -> false
        }
    }

    fun reset() {
        active = false
        pending = false
        restoreNativeAfterSession = false
    }

    private fun finishSession() {
        pending = false
        if (restoreNativeAfterSession) restoreNative()
        restoreNativeAfterSession = false
    }
}
