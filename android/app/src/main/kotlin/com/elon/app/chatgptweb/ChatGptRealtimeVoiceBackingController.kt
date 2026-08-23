package com.elon.app.chatgptweb

import android.webkit.WebView
import com.elon.app.beginWebChatBackgroundInteraction
import com.elon.app.showWebChatBackgroundSurface

internal class ChatGptRealtimeVoiceBackingController(
    private val ensureInitialized: () -> Unit,
    private val webView: () -> WebView?,
    private val surfaceMode: ChatGptWebSurfaceModeController,
    private val requestExecution: () -> Unit,
    private val requestConversationSnapshot: () -> Unit,
    private val schedule: (Runnable, Long) -> Unit,
    private val conversationSnapshotRevision: () -> Long,
    private val conversationRecoveredSince: (Long) -> Boolean,
) {
    private var active = false
    private val recoveryGate = ChatGptRealtimeVoiceRecoveryGate()

    fun isActive(): Boolean = active

    fun begin(): Boolean {
        ensureInitialized()
        val view = webView() ?: return false
        recoveryGate.invalidate()
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
        val recoveryToken = recoveryGate.arm(
            snapshotRevision = conversationSnapshotRevision(),
            reloadAllowed = !gracefulExit,
        )
        view.showWebChatBackgroundSurface()
        requestExecution()
        requestConversationSnapshot()
        schedule(Runnable {
            if (active || !recoveryGate.isCurrent(recoveryToken)) return@Runnable
            requestExecution()
            requestConversationSnapshot()
        }, SNAPSHOT_SETTLE_DELAY_MS)
        if (gracefulExit) return
        schedule(Runnable {
            if (
                active ||
                !recoveryGate.shouldReload(
                    recoveryToken,
                    conversationRecoveredSince(recoveryToken.snapshotRevision),
                )
            ) return@Runnable
            webView()?.reload()
            requestExecution()
        }, INTERRUPTED_RECOVERY_TIMEOUT_MS)
    }

    fun release() {
        active = false
        recoveryGate.invalidate()
    }

    private companion object {
        const val SNAPSHOT_SETTLE_DELAY_MS = 600L
        const val INTERRUPTED_RECOVERY_TIMEOUT_MS = 3_000L
    }
}
