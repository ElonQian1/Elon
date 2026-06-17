package com.elon.chatvoice

import java.io.File

data class ChatVoiceError(
    val code: String,
    val message: String,
    val cause: Throwable? = null,
)

data class SpeechTranscript(
    val text: String,
    val isFinal: Boolean,
    val source: SpeechSource,
)

enum class SpeechSource {
    SYSTEM_ASR,
    SERVER_ASR,
}

data class ServerAsrOptions(
    val language: String? = "auto",
    val beamSize: Int? = 5,
    val vadFilter: Boolean? = true,
    val conditionOnPreviousText: Boolean? = false,
)

data class ServerAsrResult(
    val text: String,
    val rawJson: String,
)

data class TtsRequest(
    val text: String,
    val voiceId: String? = null,
    val emotionId: String? = null,
    val intensity: Float? = null,
    val agentName: String? = null,
)

data class RecordedVoice(
    val file: File,
    val durationSeconds: Int,
    val mimeType: String = "audio/mp4",
)
