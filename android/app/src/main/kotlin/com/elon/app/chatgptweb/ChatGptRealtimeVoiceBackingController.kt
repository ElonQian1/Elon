package com.elon.app.chatgptweb

import android.content.Context
import android.webkit.WebView
import com.elon.app.BuildConfig
import com.elon.app.beginWebChatRealtimeVoiceInteraction
import com.elon.app.showWebChatBackgroundSurface

internal class ChatGptRealtimeVoiceBackingController(
    private val context: Context,
    private val ensureInitialized: () -> Unit,
    private val webView: () -> WebView?,
    private val surfaceMode: ChatGptWebSurfaceModeController,
    private val requestExecution: () -> Unit,
    private val requestPrivateConversationSnapshot: () -> Unit,
    private val requestConversationSnapshot: () -> Unit,
    private val schedule: (Runnable, Long) -> Unit,
    private val conversationSnapshotRevision: () -> Long,
    private val conversationRecoveredSince: (Long) -> Boolean,
) {
    private var active = false
    private val recoveryGate = ChatGptRealtimeVoiceRecoveryGate()
    private val privateVoiceRelay = ChatGptWebPrivateVoiceRelayGateway(webView, schedule)
    private var nativeResearch: ChatGptWebNativeVoiceResearchController? = null

    fun isActive(): Boolean = active

    fun exchangePrivateVoiceOffer(
        offer: String,
        onComplete: (ChatGptWebPrivateVoiceRelayResult) -> Unit,
    ): Boolean = privateVoiceRelay.exchange(offer, onComplete)

    fun beginNativePrivateVoiceResearch(
        onState: (ChatGptWebNativeVoiceState) -> Unit,
    ): Boolean {
        if (!BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED) return false
        ensureInitialized()
        if (webView() == null) return false
        active = true
        surfaceMode.select(ChatGptWebPresentationMode.NATIVE)
        requestExecution()
        val controller = nativeResearch ?: ChatGptWebNativeVoiceResearchController(
            context = context,
            relay = privateVoiceRelay,
            schedule = schedule,
            onState = { state ->
                if (
                    state.phase == ChatGptWebNativeVoicePhase.FAILED ||
                    state.phase == ChatGptWebNativeVoicePhase.CLOSED
                ) {
                    active = false
                }
                onState(state)
            },
        ).also { nativeResearch = it }
        return controller.start().also { accepted ->
            if (!accepted) active = false
        }
    }

    fun muteNativePrivateVoiceResearch(muted: Boolean): Boolean =
        nativeResearch?.setMuted(muted) == true

    fun begin(): Boolean {
        ensureInitialized()
        val view = webView() ?: return false
        recoveryGate.invalidate()
        active = true
        surfaceMode.select(ChatGptWebPresentationMode.NATIVE)
        if (BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED) {
            view.evaluateJavascript(
                "window.__elonChatGptRealtimeVoiceResearch?.activate?.();",
                null,
            )
        }
        view.beginWebChatRealtimeVoiceInteraction()
        requestExecution()
        return true
    }

    fun restoreAfterHostResume() {
        if (!active) return
        if (surfaceMode.isSkin()) {
            surfaceMode.apply()
        } else {
            webView()?.beginWebChatRealtimeVoiceInteraction()
        }
    }

    fun end(gracefulExit: Boolean) {
        if (!active) return
        active = false
        nativeResearch?.close()
        val view = webView() ?: return
        val recoveryToken = recoveryGate.arm(
            snapshotRevision = conversationSnapshotRevision(),
            reloadAllowed = !gracefulExit,
        )
        view.showWebChatBackgroundSurface()
        requestExecution()
        requestPrivateConversationSnapshot()
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
        nativeResearch?.close()
        nativeResearch = null
        privateVoiceRelay.cancel()
    }

    private companion object {
        const val SNAPSHOT_SETTLE_DELAY_MS = 600L
        const val INTERRUPTED_RECOVERY_TIMEOUT_MS = 3_000L
    }
}
