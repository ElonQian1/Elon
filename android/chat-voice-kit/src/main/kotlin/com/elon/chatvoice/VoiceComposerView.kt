package com.elon.chatvoice

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.text.InputType
import android.util.AttributeSet
import android.view.Gravity
import android.view.MotionEvent
import android.view.ViewGroup
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView

class VoiceComposerView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0,
) : LinearLayout(context, attrs, defStyleAttr) {
    private val main = Handler(Looper.getMainLooper())
    private val toggleButton = TextView(context)
    private val contentFrame = FrameLayout(context)
    private val editText = EditText(context)
    private val holdButton = TextView(context)
    private val plusButton = TextView(context)
    private var config = VoiceComposerConfig()
    private var callbacks: VoiceComposerCallbacks = object : VoiceComposerCallbacks {}
    private var inputMode = VoiceComposerInputMode.TEXT
    private var state = VoiceComposerState.IDLE
    private var currentZone = config.releaseZone
    private var holdStartedAtMs = 0L
    private var permissionBlocked = false
    private var suppressCancelCallback = false
    private var transcriber = newTranscriber(config)
    private var holdController = newHoldController(config)

    init {
        orientation = HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        buildChildren()
        applyConfig(config)
    }

    fun applyConfig(next: VoiceComposerConfig) {
        transcriber.release()
        main.removeCallbacksAndMessages(null)
        config = next
        currentZone = next.releaseZone
        transcriber = newTranscriber(next)
        holdController = newHoldController(next).also { it.attachTo(holdButton) }
        applyStyle()
        updateModeUi()
        showState(VoiceComposerState.IDLE, next.copy.holdToTalk)
    }

    fun setCallbacks(next: VoiceComposerCallbacks?) {
        callbacks = next ?: object : VoiceComposerCallbacks {}
    }

    fun setInputMode(mode: VoiceComposerInputMode) {
        if (inputMode == mode) return
        inputMode = mode
        updateModeUi()
        callbacks.onModeChanged(mode)
    }

    fun getInputMode(): VoiceComposerInputMode = inputMode

    fun setText(text: String) {
        editText.setText(text)
        editText.setSelection(editText.text?.length ?: 0)
    }

    fun getText(): String = editText.text?.toString().orEmpty()

    fun submitText() {
        val text = getText().trim()
        if (text.isEmpty()) return
        editText.text?.clear()
        callbacks.onTextSubmit(text)
    }

    fun setTtsPlaying(playing: Boolean) {
        if (playing) {
            showState(VoiceComposerState.TTS_PLAYING, config.copy.ttsPlaying)
        } else {
            showState(VoiceComposerState.IDLE, config.copy.holdToTalk)
        }
    }

    fun resetVoiceState() {
        permissionBlocked = false
        currentZone = config.releaseZone
        showState(VoiceComposerState.IDLE, config.copy.holdToTalk)
    }

    fun release() {
        main.removeCallbacksAndMessages(null)
        holdController.cancelActiveHold()
        transcriber.release()
    }

    private fun buildChildren() {
        addView(toggleButton, LayoutParams(dp(44), dp(44)))
        addView(
            contentFrame,
            LayoutParams(0, dp(44), 1f).apply {
                leftMargin = dp(8)
                rightMargin = dp(8)
            },
        )
        contentFrame.addView(
            editText,
            FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT),
        )
        contentFrame.addView(
            holdButton,
            FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT),
        )
        addView(plusButton, LayoutParams(dp(44), dp(44)))
        toggleButton.setOnClickListener {
            setInputMode(if (inputMode == VoiceComposerInputMode.TEXT) VoiceComposerInputMode.VOICE else VoiceComposerInputMode.TEXT)
        }
        plusButton.setOnClickListener { callbacks.onPlusClick() }
        editText.imeOptions = EditorInfo.IME_ACTION_SEND
        editText.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
        editText.maxLines = 4
        editText.setSingleLine(false)
        editText.setOnEditorActionListener { _, actionId, event ->
            val keyboardSend = actionId == EditorInfo.IME_ACTION_SEND
            val enterSend = event?.action == MotionEvent.ACTION_DOWN && event.keyCode == android.view.KeyEvent.KEYCODE_ENTER
            if (keyboardSend || enterSend) {
                submitText()
                true
            } else {
                false
            }
        }
        holdController.attachTo(holdButton)
    }

    private fun applyStyle() {
        val style = config.style
        minimumHeight = dp(style.minHeightDp)
        setPadding(
            dp(style.horizontalPaddingDp),
            dp(style.verticalPaddingDp),
            dp(style.horizontalPaddingDp),
            dp(style.verticalPaddingDp),
        )
        setBackgroundColor(style.containerBackgroundColor)
        applyIconButton(toggleButton)
        applyIconButton(plusButton)
        updateLayoutMetrics(style)
        editText.gravity = Gravity.CENTER_VERTICAL
        editText.setTextColor(style.textColor)
        editText.setHintTextColor(style.hintColor)
        editText.textSize = 16f
        editText.background = rounded(style.fieldBackgroundColor, style.fieldCornerRadiusDp)
        editText.setPadding(dp(16), 0, dp(16), 0)
        holdButton.gravity = Gravity.CENTER
        holdButton.typeface = Typeface.DEFAULT_BOLD
        holdButton.textSize = 16f
        holdButton.setTextColor(style.textColor)
        holdButton.background = rounded(style.fieldBackgroundColor, style.fieldCornerRadiusDp)
        updateButtonContent(plusButton, config.copy.plus, style.icons.plus)
    }

    private fun applyIconButton(button: TextView) {
        button.gravity = Gravity.CENTER
        button.textSize = 14f
        button.typeface = Typeface.DEFAULT_BOLD
        button.setTextColor(config.style.iconColor)
        button.background = rounded(config.style.iconBackgroundColor, config.style.iconButtonSizeDp / 2)
    }

    private fun updateModeUi() {
        val voiceMode = inputMode == VoiceComposerInputMode.VOICE
        editText.visibility = if (voiceMode) GONE else VISIBLE
        holdButton.visibility = if (voiceMode) VISIBLE else GONE
        editText.hint = config.copy.textHint
        if (voiceMode) {
            updateButtonContent(toggleButton, config.copy.keyboardToggle, config.style.icons.keyboard)
        } else {
            updateButtonContent(toggleButton, config.copy.voiceToggle, config.style.icons.voice)
        }
        holdButton.text = stateText(state)
    }

    private fun newHoldController(next: VoiceComposerConfig): HoldToTalkController =
        HoldToTalkController(
            callbacks = object : HoldToTalkController.Callbacks {
                override fun onHoldPending() {
                    permissionBlocked = false
                    currentZone = next.releaseZone
                    showState(VoiceComposerState.PREPARING, next.copy.preparing)
                }

                override fun onHoldStart() {
                    if (!hasRecordAudioPermission()) {
                        permissionBlocked = true
                        val error = ChatVoiceError("record_audio_denied", next.copy.permissionDenied)
                        next.eventSink?.onVoiceEvent(ChatVoiceEvent.Error(error))
                        showState(VoiceComposerState.PERMISSION_DENIED, next.copy.permissionDenied)
                        callbacks.onPermissionRequired()
                        callbacks.onVoiceError(error)
                        return
                    }
                    holdStartedAtMs = SystemClock.elapsedRealtime()
                    currentZone = next.releaseZone
                    next.eventSink?.onVoiceEvent(ChatVoiceEvent.Start)
                    callbacks.onVoicePressStart()
                    transcriber.start(transcriberListener(), preferOffline = next.preferOfflineAsr)
                }

                override fun onCancelZoneChanged(inCancelZone: Boolean) {
                    currentZone = if (inCancelZone) ChatVoiceZone.CANCEL else next.releaseZone
                    if (inCancelZone) {
                        showState(VoiceComposerState.CANCELING, next.copy.canceling)
                    } else {
                        showState(VoiceComposerState.RECORDING, next.copy.recording)
                    }
                    next.eventSink?.onVoiceEvent(
                        ChatVoiceEvent.ZoneChanged(
                            currentZone,
                            ChatVoiceInteractionContract.releaseText(next.chatMode, currentZone),
                        )
                    )
                }

                override fun onHoldRelease() {
                    if (permissionBlocked) {
                        resetVoiceState()
                        return
                    }
                    releaseHold()
                }

                override fun onHoldCancel() {
                    cancelHold()
                }
            },
            options = next.holdOptions,
            mode = next.chatMode,
        )

    private fun releaseHold() {
        val durationMs = SystemClock.elapsedRealtime() - holdStartedAtMs
        if (durationMs < config.holdOptions.minRecordDurationMs) {
            showTooShort()
            return
        }
        callbacks.onVoiceReleased(currentZone)
        showState(VoiceComposerState.PROCESSING, config.copy.processing)
        transcriber.stop()
    }

    private fun cancelHold() {
        val shouldNotify = !suppressCancelCallback
        suppressCancelCallback = true
        transcriber.cancel()
        if (shouldNotify) callbacks.onVoiceCanceled()
        main.post { suppressCancelCallback = false }
        resetVoiceState()
    }

    private fun showTooShort() {
        suppressCancelCallback = true
        transcriber.cancel()
        config.eventSink?.onVoiceEvent(
            ChatVoiceEvent.TooShort(
                config.holdOptions.minRecordDurationMs,
                config.holdOptions.minVoiceBytes,
            )
        )
        showState(VoiceComposerState.TOO_SHORT, config.copy.tooShort)
        callbacks.onVoiceCanceled()
        main.post { suppressCancelCallback = false }
        main.postDelayed({ resetVoiceState() }, 700L)
    }

    private fun transcriberListener(): SystemSpeechTranscriber.Listener =
        object : SystemSpeechTranscriber.Listener {
            override fun onReady() {
                showState(VoiceComposerState.RECORDING, config.copy.recording)
            }

            override fun onVolume(value: Float) {
                callbacks.onVoiceVolume(value)
            }

            override fun onPartial(transcript: SpeechTranscript) {
                callbacks.onVoicePartial(transcript)
            }

            override fun onFinal(transcript: SpeechTranscript) {
                callbacks.onVoiceRecognized(transcript, currentZone)
                resetVoiceState()
            }

            override fun onCanceled() {
                if (!suppressCancelCallback) callbacks.onVoiceCanceled()
                suppressCancelCallback = false
            }

            override fun onError(error: ChatVoiceError) {
                val permissionError = error.code == "record_audio_denied" || error.code == "system_asr_9"
                val text = if (permissionError) config.copy.permissionDenied else error.message.ifBlank { config.copy.recognitionFailed }
                showState(if (permissionError) VoiceComposerState.PERMISSION_DENIED else VoiceComposerState.ERROR, text)
                if (permissionError) callbacks.onPermissionRequired()
                callbacks.onVoiceError(error)
                main.postDelayed({ resetVoiceState() }, 900L)
            }
        }

    private fun newTranscriber(next: VoiceComposerConfig): SystemSpeechTranscriber =
        SystemSpeechTranscriber(context, next.languageTag, next.eventSink)

    private fun showState(nextState: VoiceComposerState, text: String = stateText(nextState)) {
        state = nextState
        holdButton.text = text
        holdButton.background = when (nextState) {
            VoiceComposerState.CANCELING -> rounded(config.style.cancelBackgroundColor, config.style.fieldCornerRadiusDp)
            VoiceComposerState.RECORDING -> rounded(config.style.accentColor, config.style.fieldCornerRadiusDp)
            VoiceComposerState.PROCESSING,
            VoiceComposerState.TTS_PLAYING -> rounded(config.style.fieldPressedColor, config.style.fieldCornerRadiusDp)
            else -> rounded(config.style.fieldBackgroundColor, config.style.fieldCornerRadiusDp)
        }
        callbacks.onStateChanged(nextState, text)
    }

    private fun updateLayoutMetrics(style: VoiceComposerStyle) {
        val iconSize = dp(style.iconButtonSizeDp)
        toggleButton.layoutParams = (toggleButton.layoutParams as LayoutParams).apply {
            width = iconSize
            height = iconSize
        }
        plusButton.layoutParams = (plusButton.layoutParams as LayoutParams).apply {
            width = iconSize
            height = iconSize
        }
        contentFrame.layoutParams = (contentFrame.layoutParams as LayoutParams).apply {
            height = iconSize
            leftMargin = dp(style.itemGapDp)
            rightMargin = dp(style.itemGapDp)
        }
    }

    private fun stateText(nextState: VoiceComposerState): String =
        when (nextState) {
            VoiceComposerState.IDLE -> config.copy.holdToTalk
            VoiceComposerState.PREPARING -> config.copy.preparing
            VoiceComposerState.RECORDING -> config.copy.recording
            VoiceComposerState.CANCELING -> config.copy.canceling
            VoiceComposerState.PROCESSING -> config.copy.processing
            VoiceComposerState.TOO_SHORT -> config.copy.tooShort
            VoiceComposerState.PERMISSION_DENIED -> config.copy.permissionDenied
            VoiceComposerState.ERROR -> config.copy.recognitionFailed
            VoiceComposerState.TTS_PLAYING -> config.copy.ttsPlaying
        }

    private fun updateButtonContent(button: TextView, fallbackText: String, drawable: android.graphics.drawable.Drawable?) {
        button.text = if (drawable == null) fallbackText else ""
        val icon = drawable?.mutate()
        icon?.setTint(config.style.iconColor)
        button.setCompoundDrawablesWithIntrinsicBounds(null, icon, null, null)
    }

    private fun hasRecordAudioPermission(): Boolean =
        context.checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED

    private fun rounded(color: Int, radiusDp: Int): GradientDrawable =
        GradientDrawable().apply {
            setColor(color)
            cornerRadius = dp(radiusDp).toFloat()
        }

    private fun dp(value: Int): Int =
        (value * resources.displayMetrics.density + 0.5f).toInt()
}
