package com.elon.chatvoice

enum class ChatVoiceMode {
    AGENT,
    FRIEND_CHAT,
}

enum class ChatVoiceZone {
    AI_REPLY,
    TRANSCRIBE,
    CANCEL,
    SEND,
}

enum class ChatVoiceListeningState {
    IDLE,
    PREPARING,
    LISTENING,
    HEARD,
    PROCESSING,
    SILENCE,
    NOISE,
    TOO_SHORT,
    ERROR,
    TTS_PLAYING,
}

data class ChatVoiceHoldOptions(
    val longPressStartDelayMs: Long = 0L,
    val minRecordDurationMs: Long = 600L,
    val minVoiceBytes: Long = 256L,
    val cancelDragUpDp: Int = 56,
    val horizontalChoiceDp: Int = 80,
    val touchChoiceHeightDp: Int = 118,
    val heardVibrationMs: Long = 30L,
    val heardPulseMs: Long = 220L,
    val countdownWarningRatio: Float = 0.75f,
)

data class ChatVoiceUiCopy(
    val holdToTalk: String = "按住 说话",
    val preparing: String = "准备中...",
    val listening: String = "正在听...",
    val heard: String = "听到了，松手发送",
    val processing: String = "识别中...",
    val silence: String = "没有检测到声音",
    val noise: String = "环境较嘈杂，请靠近手机说话",
    val tooShort: String = "语音太短，请轻触再试",
    val recognitionFailed: String = "识别失败，请重试",
    val releaseAiReply: String = "松开 AI回复",
    val releaseTranscribe: String = "松开 转文字",
    val releaseCancel: String = "松开 取消",
    val releaseSend: String = "松开 发送",
    val slideToAiReply: String = "滑到这 AI回复",
    val aiReplying: String = "AI回复中...",
    val transcribing: String = "转文字中...",
    val sending: String = "发送中...",
    val ttsPlaying: String = "语音播放中...",
)

data class ChatVoiceUiTokens(
    val overlayScrim: String = "#CC000000",
    val bubbleNormal: String = "#58BE6A",
    val bubbleCancel: String = "#E65A5A",
    val waveBar: String = "#2C6C52",
    val trayOuter: String = "#70575757",
    val trayInner: String = "#8A707070",
    val trayHighlight: String = "#48FFFFFF",
    val textDefault: String = "#DDEDEDED",
    val textTranscribe: String = "#EAF7F0",
    val textCancel: String = "#FFE3E3",
    val countdownNormal: String = "#60FFFFFF",
    val countdownWarning: String = "#FFCC44",
)

object ChatVoiceInteractionContract {
    val holdOptions = ChatVoiceHoldOptions()
    val copy = ChatVoiceUiCopy()
    val tokens = ChatVoiceUiTokens()

    fun defaultZone(mode: ChatVoiceMode): ChatVoiceZone =
        if (mode == ChatVoiceMode.FRIEND_CHAT) ChatVoiceZone.SEND else ChatVoiceZone.AI_REPLY

    fun stateText(state: ChatVoiceListeningState, copy: ChatVoiceUiCopy = this.copy): String =
        when (state) {
            ChatVoiceListeningState.IDLE -> copy.holdToTalk
            ChatVoiceListeningState.PREPARING -> copy.preparing
            ChatVoiceListeningState.LISTENING -> copy.listening
            ChatVoiceListeningState.HEARD -> copy.heard
            ChatVoiceListeningState.PROCESSING -> copy.processing
            ChatVoiceListeningState.SILENCE -> copy.silence
            ChatVoiceListeningState.NOISE -> copy.noise
            ChatVoiceListeningState.TOO_SHORT -> copy.tooShort
            ChatVoiceListeningState.ERROR -> copy.recognitionFailed
            ChatVoiceListeningState.TTS_PLAYING -> copy.ttsPlaying
        }

    fun releaseText(
        mode: ChatVoiceMode,
        zone: ChatVoiceZone,
        copy: ChatVoiceUiCopy = this.copy,
    ): String =
        when (zone) {
            ChatVoiceZone.AI_REPLY -> if (mode == ChatVoiceMode.FRIEND_CHAT) copy.slideToAiReply else copy.releaseAiReply
            ChatVoiceZone.TRANSCRIBE -> copy.releaseTranscribe
            ChatVoiceZone.CANCEL -> copy.releaseCancel
            ChatVoiceZone.SEND -> copy.releaseSend
        }

    fun statusText(
        mode: ChatVoiceMode,
        zone: ChatVoiceZone,
        state: ChatVoiceListeningState,
        hasTranscript: Boolean,
        copy: ChatVoiceUiCopy = this.copy,
    ): String {
        val isDefaultZone = zone == defaultZone(mode)
        if (!hasTranscript && isDefaultZone) return stateText(state, copy)
        return releaseText(mode, zone, copy)
    }

    fun zoneFromOverlayTouch(
        mode: ChatVoiceMode,
        localX: Float,
        localY: Float,
        widthPx: Int,
        heightPx: Int,
        initialRawY: Float,
        currentRawY: Float,
        density: Float,
        options: ChatVoiceHoldOptions = holdOptions,
    ): ChatVoiceZone {
        if (widthPx <= 0 || heightPx <= 0) return defaultZone(mode)
        val chooseTop = heightPx - options.touchChoiceHeightDp * density
        if (mode == ChatVoiceMode.FRIEND_CHAT) {
            val movedUpEnough = currentRawY < initialRawY - options.cancelDragUpDp * density
            if (!movedUpEnough) return ChatVoiceZone.SEND
        }
        if (localY >= chooseTop) return defaultZone(mode)
        return when {
            localX < widthPx * 0.34f -> ChatVoiceZone.CANCEL
            localX > widthPx * 0.66f -> ChatVoiceZone.TRANSCRIBE
            else -> ChatVoiceZone.AI_REPLY
        }
    }

    fun zoneFromHorizontalDelta(
        mode: ChatVoiceMode,
        deltaXPx: Float,
        density: Float,
        options: ChatVoiceHoldOptions = holdOptions,
    ): ChatVoiceZone {
        val threshold = options.horizontalChoiceDp * density
        return when {
            deltaXPx < -threshold -> ChatVoiceZone.CANCEL
            deltaXPx > threshold -> ChatVoiceZone.TRANSCRIBE
            else -> defaultZone(mode)
        }
    }
}
