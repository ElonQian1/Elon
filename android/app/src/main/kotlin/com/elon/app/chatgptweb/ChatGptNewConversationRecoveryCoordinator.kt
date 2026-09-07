package com.elon.app.chatgptweb

import android.os.Handler
import android.os.Looper

internal class ChatGptNewConversationRecoveryCoordinator(
    private val requestSnapshot: () -> Unit,
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
        when (ChatGptNewConversationRecoveryPolicy.action(
            navigationActive = navigationActive(),
            loading = loading(),
            composerReady = composerReady(),
        )) {
            ChatGptNewConversationRecoveryAction.NONE -> Unit
            ChatGptNewConversationRecoveryAction.REQUEST_SNAPSHOT -> {
                interactionRequested()
                requestSnapshot()
            }
        }
    }

    private companion object {
        const val RECOVERY_DELAY_MS = 3_000L
    }
}
