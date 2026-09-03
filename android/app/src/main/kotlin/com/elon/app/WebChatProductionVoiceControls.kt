package com.elon.app

import android.graphics.Color
import android.content.res.ColorStateList
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.InsetDrawable
import android.os.SystemClock
import android.view.View
import android.widget.ImageButton

internal data class WebChatProductionVoicePresentation(
    val dictation: WebChatProductionComposerCommand?,
    val dictationCancel: WebChatProductionComposerCommand?,
    val realtimeVoice: WebChatProductionComposerCommand?,
)

internal data class WebChatProductionDictationPresentation(
    val active: Boolean,
    val inputHint: String?,
)

internal const val WEB_CHAT_REALTIME_VOICE_HIDDEN_TAG = "web-chat-realtime-voice-hidden"

internal object WebChatProductionVoicePresentationPolicy {
    fun resolve(
        provider: WebChatProviderIdentity,
        streaming: Boolean,
        dictationActive: Boolean,
    ): WebChatProductionVoicePresentation {
        val commands = WebChatProductionComposerCommandCatalog.resolve(
            provider = provider,
            streaming = streaming,
            dictationActive = dictationActive,
        )
        return WebChatProductionVoicePresentation(
            dictation = commands.firstOrNull { it.action in DICTATION_ACTIONS },
            dictationCancel = commands.firstOrNull { it.action == CANCEL_DICTATION_ACTION },
            realtimeVoice = commands.firstOrNull { it.action == REALTIME_VOICE_ACTION },
        )
    }

    private val DICTATION_ACTIONS = setOf(
        "chatgpt_start_dictation",
        "chatgpt_submit_dictation",
    )
    private const val CANCEL_DICTATION_ACTION = "chatgpt_cancel_dictation"
    private const val REALTIME_VOICE_ACTION = "chatgpt_start_realtime_voice"
}

internal class WebChatProductionVoiceControls(
    private val dp: (Int) -> Int,
    private val inputComposerViews: () -> MainInputComposerViews?,
    private val executeCommand: (
        WebChatProviderIdentity,
        WebChatProductionComposerCommand,
    ) -> Boolean,
    private val privateDictation: WebChatPrivateDictationPort =
        WebChatUnavailablePrivateDictationPort,
    private val sharedDictation: WebChatNativeDictationPort,
    private val onNativeStateChanged: () -> Unit,
    private val readDraft: () -> String,
    private val writeDraft: (String) -> Unit,
) {
    private val domSession = WebChatDomDictationSession(SystemClock::elapsedRealtime)

    fun dictationPresentation(
        officialActive: Boolean,
        officialCaptureActive: Boolean = officialActive,
    ): WebChatProductionDictationPresentation {
        val privateState = privateDictation.state()
        val sharedState = sharedDictation.state()
        val domState = domSession.state(officialActive, readDraft(), officialCaptureActive)
        val state = if (privateState.active) privateState else sharedState
        val hint = when {
            domState.phase == WebChatDomDictationPhase.STARTING -> "正在启动语音输入…"
            domState.startFailed -> "语音输入未启动，请取消后重试"
            domState.phase == WebChatDomDictationPhase.REVIEW -> "听写已结束，请确认或取消"
            domState.phase == WebChatDomDictationPhase.SUBMITTING -> "正在完成语音输入…"
            domState.phase == WebChatDomDictationPhase.CANCELLING -> "正在取消语音输入…"
            state.phase == WebChatNativeDictationPhase.STARTING -> "正在准备语音输入…"
            state.phase == WebChatNativeDictationPhase.LISTENING -> "正在听写，点蓝色勾完成"
            state.phase == WebChatNativeDictationPhase.PROCESSING -> "正在完成语音输入…"
            else ->
                if (officialActive) "正在听写，完成后不会自动发送" else null
        }
        return WebChatProductionDictationPresentation(
            officialActive || state.active || domState.controlsActive,
            hint,
        )
    }

    fun onDomCommandResult(action: String, ok: Boolean) {
        if (action !in DOM_RESULT_ACTIONS) return
        domSession.commandResult(action, ok)
        onNativeStateChanged()
    }

    fun render(
        provider: WebChatProviderIdentity,
        streaming: Boolean,
        officialDictationActive: Boolean,
        officialDictationCaptureActive: Boolean = officialDictationActive,
    ) {
        val views = inputComposerViews() ?: return
        val privateState = privateDictation.state()
        val sharedState = sharedDictation.state()
        val domState = domSession.state(
            officialDictationActive,
            readDraft(),
            officialDictationCaptureActive,
        )
        val presentation = WebChatProductionVoicePresentationPolicy.resolve(
            provider = provider,
            streaming = streaming,
            dictationActive = officialDictationActive,
        )
        renderRealtimeVoice(
            views,
            provider,
            presentation.realtimeVoice,
            presentation.dictationCancel,
            privateState,
            sharedState,
            domState,
        )
        renderDictation(
            views = views,
            provider = provider,
            command = presentation.dictation,
            streaming = streaming,
            officialDictationActive = officialDictationActive,
            privateState = privateState,
            sharedState = sharedState,
            domState = domState,
        )
    }

    fun restoreLocalVoiceInput() {
        domSession.reset()
        privateDictation.cancel()
        val views = inputComposerViews() ?: return
        views.inputModeButton.apply {
            tag = null
            isEnabled = true
            alpha = 1f
            background = ColorDrawable(Color.TRANSPARENT)
            imageTintList = null
            setImageResource(R.drawable.ic_input_voice_wave_new)
            setPadding(dp(9), dp(9), dp(9), dp(9))
            contentDescription = LOCAL_VOICE_DESCRIPTION
            setOnClickListener { views.toggleLocalVoiceMode() }
        }
        views.webDictationButton.apply {
            visibility = View.GONE
            isActivated = false
            tag = null
            background = ColorDrawable(Color.TRANSPARENT)
            imageTintList = null
            setImageResource(R.drawable.ic_web_chat_dictation)
            setPadding(dp(8), dp(9), dp(8), dp(9))
            contentDescription = UNBOUND_DICTATION_DESCRIPTION
            setOnClickListener(null)
        }
    }

    private fun renderRealtimeVoice(
        views: MainInputComposerViews,
        provider: WebChatProviderIdentity,
        command: WebChatProductionComposerCommand?,
        cancelDictation: WebChatProductionComposerCommand?,
        privateState: WebChatNativeDictationState,
        sharedState: WebChatNativeDictationState,
        domState: WebChatDomDictationState,
    ) {
        if (privateState.active) {
            renderDictationCancel(views, PRIVATE_CANCEL_SELECTOR) {
                this@WebChatProductionVoiceControls.privateDictation.cancel()
            }
            return
        }
        if (sharedState.active) {
            renderDictationCancel(views, SHARED_CANCEL_SELECTOR) {
                this@WebChatProductionVoiceControls.sharedDictation.cancel()
            }
            return
        }
        if (domState.reviewPending) {
            renderDictationCancel(views, DOM_REVIEW_CANCEL_SELECTOR) {
                domSession.cancelReview()?.let(writeDraft)
                onNativeStateChanged()
            }
            return
        }
        if (cancelDictation != null) {
            renderDictationCancel(
                views = views,
                selector = cancelDictation.nativeSelector,
                enabled = !domState.finishPending,
            ) { finishDomDictation(provider, cancelDictation, WebChatDomDictationSession.CANCEL_ACTION) }
            return
        }
        if (command == null) {
            views.inputModeButton.apply {
                tag = if (provider.supports(WebChatProviderCapability.REALTIME_VOICE)) {
                    WEB_CHAT_REALTIME_VOICE_HIDDEN_TAG
                } else {
                    null
                }
                isEnabled = true
                alpha = 1f
                background = ColorDrawable(Color.TRANSPARENT)
                imageTintList = null
                setImageResource(R.drawable.ic_input_voice_wave_new)
                setPadding(dp(9), dp(9), dp(9), dp(9))
                contentDescription = LOCAL_VOICE_DESCRIPTION
                setOnClickListener { views.toggleLocalVoiceMode() }
            }
            return
        }
        views.inputModeButton.apply {
            tag = command.nativeSelector
            isEnabled = true
            alpha = 1f
            background = InsetDrawable(
                GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(Color.parseColor(REALTIME_VOICE_BLUE))
                },
                dp(3),
            )
            imageTintList = ColorStateList.valueOf(Color.WHITE)
            setImageResource(R.drawable.ic_input_voice)
            setPadding(dp(9), dp(9), dp(9), dp(9))
            contentDescription = command.nativeSelector
            setOnClickListener { executeCommand(provider, command) }
        }
    }

    private fun renderDictation(
        views: MainInputComposerViews,
        provider: WebChatProviderIdentity,
        command: WebChatProductionComposerCommand?,
        streaming: Boolean,
        officialDictationActive: Boolean,
        privateState: WebChatNativeDictationState,
        sharedState: WebChatNativeDictationState,
        domState: WebChatDomDictationState,
    ) {
        val sharedAvailable = provider.supports(WebChatProviderCapability.DICTATION) &&
            !streaming && !officialDictationActive
        val domStartAvailable = command?.action == DOM_START_COMMAND_ACTION
        val startAvailable = !streaming && !domState.controlsActive &&
            (privateDictation.ready() || sharedAvailable || domStartAvailable)
        val visible = startAvailable || command != null || privateState.active ||
            sharedState.active || domState.controlsActive
        val finishing = privateState.active || sharedState.active ||
            domState.canFinish || domState.finishPending
        views.webDictationButton.apply {
            isActivated = visible
            isEnabled = visible &&
                domState.phase != WebChatDomDictationPhase.STARTING &&
                !domState.startFailed &&
                !domState.finishPending
            alpha = if (isEnabled) 1f else 0.62f
            tag = when {
                privateState.active -> PRIVATE_SUBMIT_SELECTOR
                sharedState.active -> SHARED_SUBMIT_SELECTOR
                domState.reviewPending -> DOM_REVIEW_SUBMIT_SELECTOR
                domState.phase == WebChatDomDictationPhase.STARTING -> DOM_STARTING_SELECTOR
                domState.startFailed -> DOM_START_FAILED_SELECTOR
                officialDictationActive && domState.canFinish -> command?.nativeSelector
                startAvailable -> DICTATION_START_SELECTOR
                else -> command?.nativeSelector
            }
            visibility = if (visible && views.inputModeButton.visibility == View.VISIBLE) {
                View.VISIBLE
            } else {
                View.GONE
            }
            background = if (finishing) {
                InsetDrawable(
                    GradientDrawable().apply {
                        shape = GradientDrawable.OVAL
                        setColor(Color.parseColor(REALTIME_VOICE_BLUE))
                    },
                    dp(3),
                )
            } else {
                ColorDrawable(Color.TRANSPARENT)
            }
            imageTintList = if (finishing) ColorStateList.valueOf(Color.WHITE) else null
            setImageResource(
                if (finishing) R.drawable.ic_web_chat_dictation_done
                else R.drawable.ic_web_chat_dictation,
            )
            setPadding(
                if (finishing) dp(10) else dp(8),
                if (finishing) dp(10) else dp(9),
                if (finishing) dp(10) else dp(8),
                if (finishing) dp(10) else dp(9),
            )
            contentDescription = tag?.toString() ?: UNBOUND_DICTATION_DESCRIPTION
            setOnClickListener(if (!visible) null else View.OnClickListener {
                when (WebChatProductionDictationRoutePolicy.resolve(
                    privateActive = this@WebChatProductionVoiceControls.privateDictation.state().active,
                    sharedActive = this@WebChatProductionVoiceControls.sharedDictation.state().active,
                    domActive = officialDictationActive || domState.reviewPending,
                    startAvailable = startAvailable,
                )) {
                    WebChatProductionDictationTapRoute.SUBMIT_PRIVATE ->
                        this@WebChatProductionVoiceControls.privateDictation.submit()
                    WebChatProductionDictationTapRoute.SUBMIT_SHARED ->
                        this@WebChatProductionVoiceControls.sharedDictation.submit()
                    WebChatProductionDictationTapRoute.SUBMIT_DOM ->
                        if (domState.reviewPending) {
                            domSession.acceptReview()
                            onNativeStateChanged()
                        } else {
                            command?.let {
                                finishDomDictation(
                                    provider,
                                    it,
                                    WebChatDomDictationSession.SUBMIT_ACTION,
                                )
                            }
                        }
                    WebChatProductionDictationTapRoute.START ->
                        startDictation(provider, command?.takeIf { it.action == DOM_START_COMMAND_ACTION })
                    WebChatProductionDictationTapRoute.NONE -> Unit
                }
            })
        }
    }

    private fun startDictation(
        provider: WebChatProviderIdentity,
        domCommand: WebChatProductionComposerCommand?,
    ) {
        WebChatDictationStartChain.start(
            privateReady = privateDictation.ready(),
            startPrivate = {
                privateDictation.start(
                    onStateChanged = { onNativeStateChanged() },
                    onUnavailableBeforeCapture = { startSharedThenDom(provider, domCommand) },
                )
            },
            startShared = { startSharedDictation(provider, domCommand) },
            startDom = { startDomDictation(provider, domCommand) },
        )
        onNativeStateChanged()
    }

    private fun startSharedThenDom(
        provider: WebChatProviderIdentity,
        domCommand: WebChatProductionComposerCommand?,
    ): Boolean {
        val accepted = startSharedDictation(provider, domCommand) ||
            startDomDictation(provider, domCommand)
        onNativeStateChanged()
        return accepted
    }

    private fun startSharedDictation(
        provider: WebChatProviderIdentity,
        domCommand: WebChatProductionComposerCommand?,
    ): Boolean = sharedDictation.start(
        onStateChanged = { onNativeStateChanged() },
        onUnavailableBeforeCapture = {
            startDomDictation(provider, domCommand).also { accepted ->
                DebugTraceStore.record(
                    "web_chat_dictation_dom_fallback",
                    mapOf("accepted" to accepted),
                )
                onNativeStateChanged()
            }
        },
    )

    private fun startDomDictation(
        provider: WebChatProviderIdentity,
        command: WebChatProductionComposerCommand?,
    ): Boolean {
        if (command?.action != DOM_START_COMMAND_ACTION || !domSession.startRequested(readDraft())) {
            return false
        }
        onNativeStateChanged()
        inputComposerViews()?.webDictationButton?.postDelayed(
            { onNativeStateChanged() },
            WebChatDomDictationSession.DEFAULT_START_TIMEOUT_MS,
        )
        return executeCommand(provider, command).also { accepted ->
            if (!accepted) domSession.commandResult(DOM_START_RESULT_ACTION, false)
            DebugTraceStore.record(
                "web_chat_dictation_dom_start",
                mapOf("accepted" to accepted),
            )
        }
    }

    private fun finishDomDictation(
        provider: WebChatProviderIdentity,
        command: WebChatProductionComposerCommand,
        resultAction: String,
    ): Boolean {
        if (!domSession.finishRequested(resultAction)) return false
        onNativeStateChanged()
        inputComposerViews()?.webDictationButton?.postDelayed(
            { onNativeStateChanged() },
            WebChatDomDictationSession.DEFAULT_FINISH_TIMEOUT_MS,
        )
        return executeCommand(provider, command).also { accepted ->
            if (!accepted) {
                domSession.commandResult(resultAction, false)
                onNativeStateChanged()
            }
            DebugTraceStore.record(
                "web_chat_dictation_dom_finish",
                mapOf(
                    "action" to resultAction,
                    "accepted" to accepted,
                ),
            )
        }
    }

    private fun renderDictationCancel(
        views: MainInputComposerViews,
        selector: String,
        enabled: Boolean = true,
        cancel: () -> Unit,
    ) {
        views.inputModeButton.apply {
            tag = selector
            isEnabled = enabled
            alpha = if (enabled) 1f else 0.62f
            background = InsetDrawable(
                GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(Color.parseColor(DICTATION_CANCEL_SURFACE))
                },
                dp(3),
            )
            imageTintList = ColorStateList.valueOf(Color.WHITE)
            setImageResource(R.drawable.ic_web_chat_dictation_cancel)
            setPadding(dp(10), dp(10), dp(10), dp(10))
            contentDescription = selector
            setOnClickListener { cancel() }
        }
    }

    private companion object {
        const val REALTIME_VOICE_BLUE = "#2F80ED"
        const val DICTATION_CANCEL_SURFACE = "#34363A"
        const val LOCAL_VOICE_DESCRIPTION = "切换语音输入"
        const val UNBOUND_DICTATION_DESCRIPTION = "web-chat-composer-command:not-bound:dictation"
        const val DICTATION_START_SELECTOR = "web-chat-composer-command:start-dictation"
        const val PRIVATE_SUBMIT_SELECTOR = "web-chat-composer-command:private:submit-dictation"
        const val PRIVATE_CANCEL_SELECTOR = "web-chat-composer-command:private:cancel-dictation"
        const val SHARED_SUBMIT_SELECTOR = "web-chat-composer-command:shared:submit-dictation"
        const val SHARED_CANCEL_SELECTOR = "web-chat-composer-command:shared:cancel-dictation"
        const val DOM_REVIEW_SUBMIT_SELECTOR = "web-chat-composer-command:dom:review:submit-dictation"
        const val DOM_REVIEW_CANCEL_SELECTOR = "web-chat-composer-command:dom:review:cancel-dictation"
        const val DOM_STARTING_SELECTOR = "web-chat-composer-command:dom:starting-dictation"
        const val DOM_START_FAILED_SELECTOR = "web-chat-composer-command:dom:start-failed-dictation"
        const val DOM_START_COMMAND_ACTION = "chatgpt_start_dictation"
        const val DOM_START_RESULT_ACTION = "start_dictation"
        val DOM_RESULT_ACTIONS = setOf(
            DOM_START_RESULT_ACTION,
            WebChatDomDictationSession.SUBMIT_ACTION,
            WebChatDomDictationSession.CANCEL_ACTION,
        )
    }
}
