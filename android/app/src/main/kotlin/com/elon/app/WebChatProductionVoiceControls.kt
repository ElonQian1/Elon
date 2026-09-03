package com.elon.app

import android.graphics.Color
import android.content.res.ColorStateList
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.InsetDrawable
import android.os.SystemClock
import android.view.HapticFeedbackConstants
import android.view.View
import android.widget.ImageButton
import android.widget.Toast

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
    private val prepareDictationCapture: () -> Unit,
    private val readDraft: () -> String,
    private val writeDraft: (String) -> Unit,
) {
    private val domSession = WebChatDomDictationSession(SystemClock::elapsedRealtime)
    private val rearmGate = WebChatDictationRearmGate(SystemClock::elapsedRealtime)
    private val modeSelector = WebChatDictationModeSelector()

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

    fun dictationActive(state: WebChatConsumerState?): Boolean = dictationPresentation(
        officialActive = state?.dictationActive == true,
        officialCaptureActive = state?.dictationCaptureActive == true,
    ).active

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
        val dictationSessionActive = privateState.active || sharedState.active ||
            domState.controlsActive || officialDictationActive
        if (rearmGate.observe(dictationSessionActive)) {
            views.webDictationButton.postDelayed(
                { onNativeStateChanged() },
                rearmGate.remainingMs().coerceAtLeast(1L),
            )
        }
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
            setOnLongClickListener(null)
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
        val providerSupportsDictation = provider.supports(WebChatProviderCapability.DICTATION)
        val startAvailable = !streaming && !domState.controlsActive &&
            rearmGate.canStart() && providerSupportsDictation
        val visible = providerSupportsDictation || command != null || privateState.active ||
            sharedState.active || domState.controlsActive
        val finishing = privateState.active || sharedState.active ||
            domState.canFinish || domState.finishPending
        val actionEnabled = when {
            privateState.active -> privateState.phase == WebChatNativeDictationPhase.LISTENING
            sharedState.active -> sharedState.phase == WebChatNativeDictationPhase.LISTENING
            domState.reviewPending -> true
            officialDictationActive -> domState.canFinish && !domState.finishPending
            else -> startAvailable
        }
        views.webDictationButton.apply {
            isActivated = visible
            isEnabled = visible && actionEnabled &&
                domState.phase != WebChatDomDictationPhase.STARTING && !domState.startFailed
            alpha = if (isEnabled) 1f else 0.62f
            tag = when {
                privateState.active -> PRIVATE_SUBMIT_SELECTOR
                sharedState.active -> SHARED_SUBMIT_SELECTOR
                domState.reviewPending -> DOM_REVIEW_SUBMIT_SELECTOR
                domState.phase == WebChatDomDictationPhase.STARTING -> DOM_STARTING_SELECTOR
                domState.startFailed -> DOM_START_FAILED_SELECTOR
                officialDictationActive && domState.canFinish -> command?.nativeSelector
                startAvailable -> startSelector(modeSelector.selected)
                else -> command?.nativeSelector
            }
            visibility = if (visible && views.inputModeButton.visibility == View.VISIBLE) {
                View.VISIBLE
            } else {
                View.GONE
            }
            background = when {
                finishing -> ovalBackground(REALTIME_VOICE_BLUE)
                modeSelector.selected == WebChatDictationMode.SHARED ->
                    ovalBackground(WORK_DICTATION_SURFACE)
                else -> ColorDrawable(Color.TRANSPARENT)
            }
            imageTintList = when {
                finishing -> ColorStateList.valueOf(Color.WHITE)
                modeSelector.selected == WebChatDictationMode.SHARED ->
                    ColorStateList.valueOf(Color.parseColor(WORK_DICTATION_TINT))
                else -> null
            }
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
            contentDescription = when (tag) {
                PRIVATE_START_SELECTOR -> "官网语音输入，长按切换模式"
                SHARED_START_SELECTOR -> "工作语音输入，长按切换模式"
                else -> tag?.toString() ?: UNBOUND_DICTATION_DESCRIPTION
            }
            setOnClickListener(if (!visible) null else View.OnClickListener {
                val route = WebChatProductionDictationRoutePolicy.resolve(
                    privateActive = this@WebChatProductionVoiceControls.privateDictation.state().active,
                    sharedActive = this@WebChatProductionVoiceControls.sharedDictation.state().active,
                    domActive = officialDictationActive || domState.reviewPending,
                    startAvailable = startAvailable,
                )
                DebugTraceStore.record(
                    "web_chat_dictation_tap",
                    mapOf(
                        "route" to route.name.lowercase(),
                        "enabled" to isEnabled,
                    ),
                )
                when (route) {
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
                        startDictation(modeSelector.selected)
                    WebChatProductionDictationTapRoute.NONE -> Unit
                }
            })
            setOnLongClickListener(
                if (privateState.active || sharedState.active || domState.controlsActive || streaming) {
                    null
                } else {
                    View.OnLongClickListener { source ->
                        source.performHapticFeedback(HapticFeedbackConstants.LONG_PRESS)
                        val selected = modeSelector.toggle()
                        DebugTraceStore.record(
                            "web_chat_dictation_mode_changed",
                            mapOf("mode" to selected.wireValue),
                        )
                        showDictationFeedback(
                            if (selected == WebChatDictationMode.PRIVATE) {
                                "已切换到官网语音输入"
                            } else {
                                "已切换到工作语音输入"
                            },
                        )
                        onNativeStateChanged()
                        true
                    }
                },
            )
        }
    }

    private fun startDictation(mode: WebChatDictationMode) {
        prepareDictationCapture()
        val accepted = when (mode) {
            WebChatDictationMode.PRIVATE -> privateDictation.ready() &&
                privateDictation.start { onNativeStateChanged() }
            WebChatDictationMode.SHARED -> sharedDictation.start { onNativeStateChanged() }
        }
        DebugTraceStore.record(
            "web_chat_dictation_explicit_start",
            mapOf("mode" to mode.wireValue, "accepted" to accepted),
        )
        if (!accepted) {
            showDictationFeedback(
                if (mode == WebChatDictationMode.PRIVATE) {
                    "官网语音正在连接，请稍后重试"
                } else {
                    "工作语音暂时不可用，请稍后重试"
                },
            )
        }
        onNativeStateChanged()
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

    private fun ovalBackground(color: String) = InsetDrawable(
        GradientDrawable().apply {
            shape = GradientDrawable.OVAL
            setColor(Color.parseColor(color))
        },
        dp(3),
    )

    private fun showDictationFeedback(message: String) {
        val context = inputComposerViews()?.webDictationButton?.context ?: return
        Toast.makeText(context, message, Toast.LENGTH_SHORT).show()
    }

    private fun startSelector(mode: WebChatDictationMode): String = when (mode) {
        WebChatDictationMode.PRIVATE -> PRIVATE_START_SELECTOR
        WebChatDictationMode.SHARED -> SHARED_START_SELECTOR
    }

    private val WebChatDictationMode.wireValue: String
        get() = name.lowercase()

    private companion object {
        const val REALTIME_VOICE_BLUE = "#2F80ED"
        const val DICTATION_CANCEL_SURFACE = "#34363A"
        const val WORK_DICTATION_SURFACE = "#25312E"
        const val WORK_DICTATION_TINT = "#A7D8C8"
        const val LOCAL_VOICE_DESCRIPTION = "切换语音输入"
        const val UNBOUND_DICTATION_DESCRIPTION = "web-chat-composer-command:not-bound:dictation"
        const val PRIVATE_START_SELECTOR = "web-chat-composer-command:private:start-dictation"
        const val SHARED_START_SELECTOR = "web-chat-composer-command:shared:start-dictation"
        const val PRIVATE_SUBMIT_SELECTOR = "web-chat-composer-command:private:submit-dictation"
        const val PRIVATE_CANCEL_SELECTOR = "web-chat-composer-command:private:cancel-dictation"
        const val SHARED_SUBMIT_SELECTOR = "web-chat-composer-command:shared:submit-dictation"
        const val SHARED_CANCEL_SELECTOR = "web-chat-composer-command:shared:cancel-dictation"
        const val DOM_REVIEW_SUBMIT_SELECTOR = "web-chat-composer-command:dom:review:submit-dictation"
        const val DOM_REVIEW_CANCEL_SELECTOR = "web-chat-composer-command:dom:review:cancel-dictation"
        const val DOM_STARTING_SELECTOR = "web-chat-composer-command:dom:starting-dictation"
        const val DOM_START_FAILED_SELECTOR = "web-chat-composer-command:dom:start-failed-dictation"
        const val DOM_START_RESULT_ACTION = "start_dictation"
        val DOM_RESULT_ACTIONS = setOf(
            DOM_START_RESULT_ACTION,
            WebChatDomDictationSession.SUBMIT_ACTION,
            WebChatDomDictationSession.CANCEL_ACTION,
        )
    }
}
