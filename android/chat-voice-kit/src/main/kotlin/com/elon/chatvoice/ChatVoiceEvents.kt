package com.elon.chatvoice

sealed class ChatVoiceEvent {
    object Start : ChatVoiceEvent()
    object Cancel : ChatVoiceEvent()
    object TtsStart : ChatVoiceEvent()
    object TtsEnd : ChatVoiceEvent()

    data class StateChanged(val state: ChatVoiceListeningState, val text: String) : ChatVoiceEvent()
    data class ZoneChanged(val zone: ChatVoiceZone, val text: String) : ChatVoiceEvent()
    data class Volume(val value: Float) : ChatVoiceEvent()
    data class PartialResult(val transcript: SpeechTranscript) : ChatVoiceEvent()
    data class FinalResult(val transcript: SpeechTranscript) : ChatVoiceEvent()
    data class Error(val error: ChatVoiceError) : ChatVoiceEvent()
    data class TooShort(val minimumDurationMs: Long, val minimumBytes: Long) : ChatVoiceEvent()
}

fun interface ChatVoiceEventSink {
    fun onVoiceEvent(event: ChatVoiceEvent)
}
