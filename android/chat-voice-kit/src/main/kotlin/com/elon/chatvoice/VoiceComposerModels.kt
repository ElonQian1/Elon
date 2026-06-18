package com.elon.chatvoice

import android.graphics.Color
import android.graphics.drawable.Drawable

enum class VoiceComposerInputMode {
    TEXT,
    VOICE,
}

enum class VoiceComposerState {
    IDLE,
    PREPARING,
    RECORDING,
    CANCELING,
    PROCESSING,
    SERVER_PROCESSING,
    TOO_SHORT,
    PERMISSION_DENIED,
    ERROR,
    TTS_PLAYING,
}

data class VoiceComposerIcons(
    val voice: Drawable? = null,
    val keyboard: Drawable? = null,
    val plus: Drawable? = null,
)

data class VoiceComposerCopy(
    val textHint: String = "输入消息",
    val voiceToggle: String = "语音",
    val keyboardToggle: String = "键盘",
    val plus: String = "+",
    val holdToTalk: String = ChatVoiceInteractionContract.copy.holdToTalk,
    val preparing: String = ChatVoiceInteractionContract.copy.preparing,
    val recording: String = ChatVoiceInteractionContract.copy.listening,
    val processing: String = ChatVoiceInteractionContract.copy.processing,
    val serverProcessing: String = "云端识别中...",
    val tooShort: String = ChatVoiceInteractionContract.copy.tooShort,
    val permissionDenied: String = "需要麦克风权限",
    val recognitionFailed: String = ChatVoiceInteractionContract.copy.recognitionFailed,
    val canceling: String = ChatVoiceInteractionContract.copy.releaseCancel,
    val ttsPlaying: String = ChatVoiceInteractionContract.copy.ttsPlaying,
)

data class VoiceComposerStyle(
    val containerBackgroundColor: Int = Color.TRANSPARENT,
    val fieldBackgroundColor: Int = Color.parseColor("#272727"),
    val fieldPressedColor: Int = Color.parseColor("#323232"),
    val cancelBackgroundColor: Int = Color.parseColor(ChatVoiceInteractionContract.tokens.bubbleCancel),
    val iconBackgroundColor: Int = Color.TRANSPARENT,
    val textColor: Int = Color.parseColor("#EDEDED"),
    val hintColor: Int = Color.parseColor("#8A8A8A"),
    val iconColor: Int = Color.parseColor("#D8D8D8"),
    val accentColor: Int = Color.parseColor(ChatVoiceInteractionContract.tokens.bubbleNormal),
    val fieldCornerRadiusDp: Int = 22,
    val iconButtonSizeDp: Int = 44,
    val minHeightDp: Int = 52,
    val horizontalPaddingDp: Int = 8,
    val verticalPaddingDp: Int = 6,
    val itemGapDp: Int = 8,
    val icons: VoiceComposerIcons = VoiceComposerIcons(),
)

data class VoiceComposerAsrConfig(
    val serverFallbackEnabled: Boolean = false,
    val serverConfig: ChatVoiceConfig? = null,
    val serverOptions: ServerAsrOptions = ServerAsrOptions(),
    val localResultTimeoutMs: Long = 4_500L,
    val deleteRecordedFileAfterResult: Boolean = true,
)

data class VoiceComposerConfig(
    val chatMode: ChatVoiceMode = ChatVoiceMode.FRIEND_CHAT,
    val releaseZone: ChatVoiceZone = ChatVoiceInteractionContract.defaultZone(chatMode),
    val languageTag: String = "zh-CN",
    val preferOfflineAsr: Boolean = false,
    val asr: VoiceComposerAsrConfig = VoiceComposerAsrConfig(),
    val holdOptions: ChatVoiceHoldOptions = ChatVoiceInteractionContract.holdOptions,
    val copy: VoiceComposerCopy = VoiceComposerCopy(),
    val style: VoiceComposerStyle = VoiceComposerStyle(),
    val eventSink: ChatVoiceEventSink? = null,
)

interface VoiceComposerCallbacks {
    fun onTextSubmit(text: String) {}
    fun onModeChanged(mode: VoiceComposerInputMode) {}
    fun onVoicePressStart() {}
    fun onVoiceVolume(value: Float) {}
    fun onVoicePartial(transcript: SpeechTranscript) {}
    fun onVoiceRecognized(transcript: SpeechTranscript, zone: ChatVoiceZone) {}
    fun onVoiceReleased(zone: ChatVoiceZone) {}
    fun onVoiceServerFallbackStarted(reason: ChatVoiceError?) {}
    fun onVoiceCanceled() {}
    fun onPermissionRequired() {}
    fun onVoiceError(error: ChatVoiceError) {}
    fun onStateChanged(state: VoiceComposerState, text: String) {}
    fun onPlusClick() {}
}
