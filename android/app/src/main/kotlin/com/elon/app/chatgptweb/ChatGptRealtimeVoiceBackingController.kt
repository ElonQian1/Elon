package com.elon.app.chatgptweb

import android.content.Context
import android.webkit.WebView
import com.elon.app.BuildConfig
import com.elon.app.WebChatManagedRealtimeVoicePhase
import com.elon.app.WebChatManagedRealtimeVoiceState
import com.elon.app.beginWebChatRealtimeVoiceInteraction
import com.elon.app.showWebChatBackgroundSurface

internal class ChatGptRealtimeVoiceBackingController(
    private val context: Context,
    private val ensureInitialized: () -> Unit,
    private val webView: () -> WebView?,
    private val surfaceMode: ChatGptWebSurfaceModeController,
    private val startOfficialVoice: () -> Boolean,
    private val requestExecution: () -> Unit,
    private val requestPrivateConversationSnapshot: () -> Unit,
    private val requestConversationSnapshot: () -> Unit,
    private val schedule: (Runnable, Long) -> Unit,
    private val conversationSnapshotRevision: () -> Long,
    private val conversationRecoveredSince: (Long) -> Boolean,
    private val onTranscript: (ChatGptWebNativeVoiceTranscriptEvent) -> Unit,
) {
    private var active = false
    private val recoveryGate = ChatGptRealtimeVoiceRecoveryGate()
    private val privateVoiceRelay = ChatGptWebPrivateVoiceRelayGateway(webView, schedule)
    private var nativeResearch: ChatGptWebNativeVoiceResearchController? = null
    private var officialFallbackPending = false
    private var transcriptRefreshGeneration = 0L
    @Volatile
    private var nativeVoiceState = ChatGptWebNativeVoiceState(ChatGptWebNativeVoicePhase.IDLE)
    private var nativeStateObserver: ((ChatGptWebNativeVoiceState) -> Unit)? = null

    fun isActive(): Boolean = active

    fun exchangePrivateVoiceOffer(
        offer: String,
        onComplete: (ChatGptWebPrivateVoiceRelayResult) -> Unit,
    ): Boolean = privateVoiceRelay.exchange(offer, onComplete)

    fun beginNativePrivateVoiceResearch(
        onState: (ChatGptWebNativeVoiceState) -> Unit,
    ): Boolean = startNativePrivateVoice(onState)

    fun startManagedRealtimeVoice(): Boolean = startNativePrivateVoice(onState = null)

    fun managedRealtimeVoiceState(): WebChatManagedRealtimeVoiceState =
        nativeVoiceState.toManagedRealtimeVoiceState(
            BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED,
        )

    fun nativePrivateVoiceState(): ChatGptWebNativeVoiceState = nativeVoiceState

    fun setManagedRealtimeVoiceMuted(muted: Boolean): Boolean =
        nativeResearch?.setMuted(muted) == true

    private fun startNativePrivateVoice(
        onState: ((ChatGptWebNativeVoiceState) -> Unit)?,
    ): Boolean {
        if (!BuildConfig.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED) return false
        ensureInitialized()
        if (webView() == null) return false
        active = true
        surfaceMode.select(ChatGptWebPresentationMode.NATIVE)
        requestExecution()
        nativeStateObserver = onState
        nativeVoiceState = ChatGptWebNativeVoiceState(ChatGptWebNativeVoicePhase.BOOTSTRAPPING)
        transcriptRefreshGeneration += 1
        val refreshToken = transcriptRefreshGeneration
        val controller = nativeResearch ?: ChatGptWebNativeVoiceResearchController(
            context = context,
            relay = privateVoiceRelay,
            schedule = schedule,
            startOfficialVoice = startOfficialVoice,
            requestOfficialFallback = ::requestOfficialFallback,
            onTranscript = onTranscript,
            onState = ::acceptNativeVoiceState,
        ).also { nativeResearch = it }
        return controller.start().also { accepted ->
            if (!accepted) {
                nativeVoiceState = ChatGptWebNativeVoiceState(
                    ChatGptWebNativeVoicePhase.FAILED,
                    code = "start_rejected",
                )
            } else {
                scheduleActiveTranscriptRefresh(refreshToken, attempt = 0)
            }
        }
    }

    fun muteNativePrivateVoiceResearch(muted: Boolean): Boolean =
        setManagedRealtimeVoiceMuted(muted)

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

    fun onUiManifestAvailable() {
        if (!officialFallbackPending) return
        if (startOfficialVoice()) officialFallbackPending = false
    }

    fun end(gracefulExit: Boolean) {
        if (!active) return
        active = false
        transcriptRefreshGeneration += 1
        officialFallbackPending = false
        nativeResearch?.close()
        nativeStateObserver = null
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
        transcriptRefreshGeneration += 1
        officialFallbackPending = false
        recoveryGate.invalidate()
        nativeResearch?.close()
        nativeResearch = null
        nativeStateObserver = null
        nativeVoiceState = ChatGptWebNativeVoiceState(ChatGptWebNativeVoicePhase.CLOSED)
        privateVoiceRelay.cancel()
    }

    private fun acceptNativeVoiceState(state: ChatGptWebNativeVoiceState) {
        nativeVoiceState = state
        nativeStateObserver?.invoke(state)
    }

    private fun scheduleActiveTranscriptRefresh(token: Long, attempt: Int) {
        schedule(Runnable {
            if (
                token != transcriptRefreshGeneration ||
                !active ||
                nativeVoiceState.phase in setOf(
                    ChatGptWebNativeVoicePhase.FAILED,
                    ChatGptWebNativeVoicePhase.CLOSED,
                    ChatGptWebNativeVoicePhase.OFFICIAL_FALLBACK,
                )
            ) return@Runnable

            val liveTranscriptObserved = nativeVoiceState.transcriptEventCount > 0
            if (!liveTranscriptObserved || attempt % PRIVATE_REFRESH_RECONCILE_INTERVAL == 0) {
                requestPrivateConversationSnapshot()
            }
            if (attempt % DOM_REFRESH_WATCHDOG_INTERVAL == 0) {
                requestConversationSnapshot()
            }
            scheduleActiveTranscriptRefresh(token, attempt + 1)
        }, if (attempt == 0) ACTIVE_TRANSCRIPT_INITIAL_DELAY_MS else ACTIVE_TRANSCRIPT_REFRESH_DELAY_MS)
    }

    private fun requestOfficialFallback(): Boolean {
        val view = webView() ?: return false
        officialFallbackPending = true
        view.reload()
        requestExecution()
        return true
    }

    private companion object {
        const val SNAPSHOT_SETTLE_DELAY_MS = 600L
        const val INTERRUPTED_RECOVERY_TIMEOUT_MS = 3_000L
        const val ACTIVE_TRANSCRIPT_INITIAL_DELAY_MS = 700L
        const val ACTIVE_TRANSCRIPT_REFRESH_DELAY_MS = 1_500L
        const val PRIVATE_REFRESH_RECONCILE_INTERVAL = 4
        const val DOM_REFRESH_WATCHDOG_INTERVAL = 8
    }
}

internal fun ChatGptWebNativeVoiceState.toManagedRealtimeVoiceState(
    enabled: Boolean,
): WebChatManagedRealtimeVoiceState {
    if (!enabled) return WebChatManagedRealtimeVoiceState.Unavailable
    val managedPhase = when (phase) {
        ChatGptWebNativeVoicePhase.IDLE -> WebChatManagedRealtimeVoicePhase.IDLE
        ChatGptWebNativeVoicePhase.BOOTSTRAPPING,
        ChatGptWebNativeVoicePhase.CREATING_OFFER,
        ChatGptWebNativeVoicePhase.RELAYING,
        ChatGptWebNativeVoicePhase.APPLYING_ANSWER,
        ChatGptWebNativeVoicePhase.CONNECTING -> WebChatManagedRealtimeVoicePhase.STARTING
        ChatGptWebNativeVoicePhase.CONNECTED -> WebChatManagedRealtimeVoicePhase.ACTIVE
        ChatGptWebNativeVoicePhase.OFFICIAL_FALLBACK ->
            WebChatManagedRealtimeVoicePhase.OFFICIAL_FALLBACK
        ChatGptWebNativeVoicePhase.FAILED -> WebChatManagedRealtimeVoicePhase.FAILED
        ChatGptWebNativeVoicePhase.CLOSED -> WebChatManagedRealtimeVoicePhase.CLOSED
    }
    val detailCode = code ?: if (
        phase == ChatGptWebNativeVoicePhase.CONNECTED && (!remoteAudio || !dataChannelOpen)
    ) {
        "media_observation_pending"
    } else {
        null
    }
    return WebChatManagedRealtimeVoiceState(managedPhase, detailCode)
}
