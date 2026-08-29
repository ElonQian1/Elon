package com.elon.app.chatgptweb

import org.json.JSONObject

internal enum class ChatGptWebNativeVoiceTranscriptSpeaker(val role: String) {
    USER("user"),
    ASSISTANT("assistant"),
}

internal enum class ChatGptWebNativeVoiceTranscriptUpdate {
    DELTA,
    FINAL,
}

internal data class ChatGptWebNativeVoiceTranscriptEvent(
    val eventId: String?,
    val streamKey: String,
    val speaker: ChatGptWebNativeVoiceTranscriptSpeaker,
    val update: ChatGptWebNativeVoiceTranscriptUpdate,
    val text: String,
)

/** Parses only bounded transcript events from the native WebRTC data channel. */
internal object ChatGptWebNativeVoiceTranscriptParser {
    private val assistantDeltaTypes = setOf(
        "response.output_audio_transcript.delta",
        "response.audio_transcript.delta",
    )
    private val assistantFinalTypes = setOf(
        "response.output_audio_transcript.done",
        "response.audio_transcript.done",
    )
    private val userDeltaTypes = setOf(
        "conversation.item.input_audio_transcription.delta",
    )
    private val userFinalTypes = setOf(
        "conversation.item.input_audio_transcription.completed",
    )

    fun parse(payload: String): ChatGptWebNativeVoiceTranscriptEvent? {
        if (payload.isBlank() || payload.length > MAX_PAYLOAD_CHARS) return null
        val root = runCatching { JSONObject(payload) }.getOrNull() ?: return null
        val value = root.optJSONObject("event") ?: root
        val type = value.optString("type").takeIf(::validToken) ?: return null
        val descriptor = descriptor(type) ?: return null
        val textField = if (descriptor.update == ChatGptWebNativeVoiceTranscriptUpdate.DELTA) {
            "delta"
        } else {
            "transcript"
        }
        val text = value.optString(textField)
            .takeIf { it.isNotEmpty() && it.length <= MAX_TRANSCRIPT_CHARS }
            ?: return null
        val streamKey = sequenceOf(
            value.optString("item_id"),
            value.optString("response_id"),
        ).firstOrNull(::validIdentifier) ?: return null
        val eventId = value.optString("event_id").takeIf(::validIdentifier)
        return ChatGptWebNativeVoiceTranscriptEvent(
            eventId = eventId,
            streamKey = streamKey,
            speaker = descriptor.speaker,
            update = descriptor.update,
            text = text,
        )
    }

    private fun descriptor(type: String): Descriptor? = when (type) {
        in assistantDeltaTypes -> Descriptor(
            ChatGptWebNativeVoiceTranscriptSpeaker.ASSISTANT,
            ChatGptWebNativeVoiceTranscriptUpdate.DELTA,
        )
        in assistantFinalTypes -> Descriptor(
            ChatGptWebNativeVoiceTranscriptSpeaker.ASSISTANT,
            ChatGptWebNativeVoiceTranscriptUpdate.FINAL,
        )
        in userDeltaTypes -> Descriptor(
            ChatGptWebNativeVoiceTranscriptSpeaker.USER,
            ChatGptWebNativeVoiceTranscriptUpdate.DELTA,
        )
        in userFinalTypes -> Descriptor(
            ChatGptWebNativeVoiceTranscriptSpeaker.USER,
            ChatGptWebNativeVoiceTranscriptUpdate.FINAL,
        )
        else -> null
    }

    private fun validToken(value: String): Boolean =
        value.length in 1..96 && value.all { it.isLetterOrDigit() || it in "._-" }

    private fun validIdentifier(value: String): Boolean =
        value.length in 1..192 && value.all { it.isLetterOrDigit() || it in "_-:." }

    private data class Descriptor(
        val speaker: ChatGptWebNativeVoiceTranscriptSpeaker,
        val update: ChatGptWebNativeVoiceTranscriptUpdate,
    )

    private const val MAX_PAYLOAD_CHARS = 256 * 1024
    private const val MAX_TRANSCRIPT_CHARS = 64 * 1024
}
