package com.elon.app

import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.InsetDrawable
import android.view.View
import android.widget.ImageButton

internal data class WebChatProductionVoicePresentation(
    val dictation: WebChatProductionComposerCommand?,
    val realtimeVoice: WebChatProductionComposerCommand?,
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
            realtimeVoice = commands.firstOrNull { it.action == REALTIME_VOICE_ACTION },
        )
    }

    private val DICTATION_ACTIONS = setOf(
        "chatgpt_start_dictation",
        "chatgpt_submit_dictation",
    )
    private const val REALTIME_VOICE_ACTION = "chatgpt_start_realtime_voice"
}

internal class WebChatProductionVoiceControls(
    private val dp: (Int) -> Int,
    private val inputComposerViews: () -> MainInputComposerViews?,
    private val executeCommand: (
        WebChatProviderIdentity,
        WebChatProductionComposerCommand,
    ) -> Boolean,
) {
    fun render(
        provider: WebChatProviderIdentity,
        streaming: Boolean,
        dictationActive: Boolean,
    ) {
        val views = inputComposerViews() ?: return
        val presentation = WebChatProductionVoicePresentationPolicy.resolve(
            provider = provider,
            streaming = streaming,
            dictationActive = dictationActive,
        )
        renderRealtimeVoice(views, provider, presentation.realtimeVoice)
        renderDictation(views, provider, presentation.dictation)
    }

    fun restoreLocalVoiceInput() {
        val views = inputComposerViews() ?: return
        views.inputModeButton.apply {
            tag = null
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_input_voice_wave_new)
            setPadding(dp(9), dp(9), dp(9), dp(9))
            contentDescription = LOCAL_VOICE_DESCRIPTION
            setOnClickListener { views.toggleLocalVoiceMode() }
        }
        views.webDictationButton.apply {
            visibility = View.GONE
            isActivated = false
            contentDescription = UNBOUND_DICTATION_DESCRIPTION
            setOnClickListener(null)
        }
    }

    private fun renderRealtimeVoice(
        views: MainInputComposerViews,
        provider: WebChatProviderIdentity,
        command: WebChatProductionComposerCommand?,
    ) {
        if (command == null) {
            views.inputModeButton.apply {
                tag = if (provider.supports(WebChatProviderCapability.REALTIME_VOICE)) {
                    WEB_CHAT_REALTIME_VOICE_HIDDEN_TAG
                } else {
                    null
                }
                background = ColorDrawable(Color.TRANSPARENT)
                setImageResource(R.drawable.ic_input_voice_wave_new)
                setPadding(dp(9), dp(9), dp(9), dp(9))
                contentDescription = LOCAL_VOICE_DESCRIPTION
                setOnClickListener { views.toggleLocalVoiceMode() }
            }
            return
        }
        views.inputModeButton.apply {
            tag = command.nativeSelector
            background = InsetDrawable(
                GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(Color.parseColor(REALTIME_VOICE_BLUE))
                },
                dp(3),
            )
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
    ) {
        views.webDictationButton.apply {
            isActivated = command != null
            visibility = if (command != null && views.inputModeButton.visibility == View.VISIBLE) {
                View.VISIBLE
            } else {
                View.GONE
            }
            contentDescription = command?.nativeSelector ?: UNBOUND_DICTATION_DESCRIPTION
            setOnClickListener(
                command?.let { resolved ->
                    View.OnClickListener { executeCommand(provider, resolved) }
                },
            )
        }
    }

    private companion object {
        const val REALTIME_VOICE_BLUE = "#2F80ED"
        const val LOCAL_VOICE_DESCRIPTION = "切换语音输入"
        const val UNBOUND_DICTATION_DESCRIPTION = "web-chat-composer-command:not-bound:dictation"
    }
}
