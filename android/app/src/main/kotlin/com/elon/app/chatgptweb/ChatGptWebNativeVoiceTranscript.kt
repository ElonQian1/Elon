package com.elon.app.chatgptweb

import org.json.JSONArray
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

/** Parses the bounded public Realtime transcript event variants. */
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

    private val privateTranscriptTypes = setOf(
        "chat_message_delta",
        "full_chat_message",
        "live_captioning_text",
        "user_transcription_text",
    )

    fun parse(payload: String): ChatGptWebNativeVoiceTranscriptEvent? {
        val value = eventObject(payload) ?: return null
        return parseEvent(value)
    }

    internal fun parseEvent(value: JSONObject): ChatGptWebNativeVoiceTranscriptEvent? {
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

    fun structuralEventType(payload: String): String? {
        val value = eventObject(payload) ?: return null
        return value.optString("type").takeIf(::validToken)
    }

    fun isTranscriptCandidateType(type: String): Boolean =
        descriptor(type) != null || type in privateTranscriptTypes

    internal fun eventObject(payload: String): JSONObject? {
        if (payload.isBlank() || payload.length > MAX_PAYLOAD_CHARS) return null
        var current = runCatching { JSONObject(payload) }.getOrNull() ?: return null
        repeat(MAX_ENVELOPE_DEPTH) {
            val type = current.optString("type")
            if (type.isNotEmpty() && type != PRIVATE_ENVELOPE_TYPE) return current
            val nested = ENVELOPE_KEYS.asSequence()
                .mapNotNull { key -> nestedObject(current.opt(key)) }
                .firstOrNull()
                ?: return current
            current = nested
        }
        return current
    }

    private fun nestedObject(value: Any?): JSONObject? = when (value) {
        is JSONObject -> value
        is JSONArray -> (0 until value.length()).asSequence()
            .mapNotNull { index -> nestedObject(value.opt(index)) }
            .firstOrNull()
        is String -> value
            .takeIf { it.length in 2..MAX_NESTED_JSON_CHARS }
            ?.let { runCatching { JSONObject(it) }.getOrNull() }
        else -> null
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

    internal fun validToken(value: String): Boolean =
        value.length in 1..96 && value.all { it.isLetterOrDigit() || it in "._-" }

    internal fun validIdentifier(value: String): Boolean =
        value.length in 1..192 && value.all { it.isLetterOrDigit() || it in "_-:." }

    private data class Descriptor(
        val speaker: ChatGptWebNativeVoiceTranscriptSpeaker,
        val update: ChatGptWebNativeVoiceTranscriptUpdate,
    )

    private const val MAX_PAYLOAD_CHARS = 256 * 1024
    private const val MAX_NESTED_JSON_CHARS = 128 * 1024
    private const val MAX_TRANSCRIPT_CHARS = 64 * 1024
    private const val MAX_ENVELOPE_DEPTH = 3
    private const val PRIVATE_ENVELOPE_TYPE = "data_message"
    private val ENVELOPE_KEYS = listOf("event", "data", "message", "payload", "body")
}

/** Stateful decoder for ChatGPT Web's private message delta and caption events. */
internal class ChatGptWebNativeVoiceTranscriptDecoder {
    private val deltaDecoder = ChatGptWebNativeVoiceJsonDeltaDecoder()
    private val previousTextByStream = LinkedHashMap<String, String>()

    fun decode(payload: String): ChatGptWebNativeVoiceTranscriptEvent? {
        val event = ChatGptWebNativeVoiceTranscriptParser.eventObject(payload) ?: return null
        ChatGptWebNativeVoiceTranscriptParser.parseEvent(event)?.let { return it }
        return when (event.optString("type")) {
            TYPE_CHAT_MESSAGE_DELTA -> decodeMessageDelta(event)
            TYPE_FULL_CHAT_MESSAGE -> decodeFullMessage(event)
            TYPE_USER_TRANSCRIPTION -> decodeDirectText(
                event,
                ChatGptWebNativeVoiceTranscriptSpeaker.USER,
            )
            TYPE_LIVE_CAPTION -> decodeDirectText(
                event,
                ChatGptWebNativeVoiceTranscriptSpeaker.ASSISTANT,
            )
            else -> null
        }
    }

    fun reset() {
        deltaDecoder.reset()
        previousTextByStream.clear()
    }

    private fun decodeMessageDelta(event: JSONObject): ChatGptWebNativeVoiceTranscriptEvent? {
        val delta = event.optJSONObject("payload")?.optJSONObject("delta") ?: return null
        val decoded = deltaDecoder.apply(delta) as? JSONObject ?: return null
        val message = decoded.optJSONObject("message") ?: return null
        return messageEvent(event, message, defaultFinal = false)
    }

    private fun decodeFullMessage(event: JSONObject): ChatGptWebNativeVoiceTranscriptEvent? {
        val message = event.optJSONObject("payload")?.optJSONObject("message") ?: return null
        return messageEvent(event, message, defaultFinal = true)
    }

    private fun messageEvent(
        event: JSONObject,
        message: JSONObject,
        defaultFinal: Boolean,
    ): ChatGptWebNativeVoiceTranscriptEvent? {
        val streamKey = sequenceOf(message.optString("id"), message.optString("message_id"))
            .firstOrNull(ChatGptWebNativeVoiceTranscriptParser::validIdentifier)
            ?: return null
        val content = message.optJSONObject("content") ?: return null
        val speaker = messageSpeaker(message, content) ?: return null
        val currentText = messageText(content)
            .takeIf { it.isNotBlank() && it.length <= MAX_TRANSCRIPT_CHARS }
            ?: return null
        val terminal = defaultFinal || message.optBoolean("end_turn") ||
            message.optString("status").lowercase() in TERMINAL_MESSAGE_STATES
        val previous = previousTextByStream[streamKey]
        val compatibleDelta = previous == null || currentText.startsWith(previous)
        val update = if (terminal || !compatibleDelta) {
            ChatGptWebNativeVoiceTranscriptUpdate.FINAL
        } else {
            ChatGptWebNativeVoiceTranscriptUpdate.DELTA
        }
        val emittedText = if (update == ChatGptWebNativeVoiceTranscriptUpdate.FINAL) {
            currentText
        } else {
            currentText.removePrefix(previous.orEmpty())
        }
        previousTextByStream[streamKey] = currentText
        trimStreams()
        if (emittedText.isEmpty() && update == ChatGptWebNativeVoiceTranscriptUpdate.DELTA) return null
        return ChatGptWebNativeVoiceTranscriptEvent(
            eventId = eventId(event),
            streamKey = streamKey,
            speaker = speaker,
            update = update,
            text = emittedText,
        )
    }

    private fun decodeDirectText(
        event: JSONObject,
        speaker: ChatGptWebNativeVoiceTranscriptSpeaker,
    ): ChatGptWebNativeVoiceTranscriptEvent? {
        val body = event.optJSONObject("payload") ?: return null
        val streamKey = sequenceOf(
            body.optString("message_id"),
            body.optString("item_id"),
            body.optString("turn_id"),
        ).firstOrNull(ChatGptWebNativeVoiceTranscriptParser::validIdentifier) ?: return null
        val text = DIRECT_TEXT_FIELDS.asSequence()
            .mapNotNull { field -> body.opt(field) as? String }
            .firstOrNull { it.isNotBlank() && it.length <= MAX_TRANSCRIPT_CHARS }
            ?: return null
        val terminal = body.optBoolean("final") ||
            body.optString("status").lowercase() in TERMINAL_MESSAGE_STATES
        return messageEvent(
            event = event,
            message = JSONObject()
                .put("id", streamKey)
                .put("author", JSONObject().put("role", speaker.role))
                .put("content", JSONObject().put("parts", JSONArray().put(text)))
                .put("end_turn", terminal),
            defaultFinal = terminal,
        )
    }

    private fun messageSpeaker(
        message: JSONObject,
        content: JSONObject,
    ): ChatGptWebNativeVoiceTranscriptSpeaker? {
        return when (message.optJSONObject("author")?.optString("role")) {
            ChatGptWebNativeVoiceTranscriptSpeaker.USER.role ->
                ChatGptWebNativeVoiceTranscriptSpeaker.USER
            ChatGptWebNativeVoiceTranscriptSpeaker.ASSISTANT.role ->
                ChatGptWebNativeVoiceTranscriptSpeaker.ASSISTANT
            else -> audioTranscriptionSpeaker(content)
        }
    }

    private fun audioTranscriptionSpeaker(
        content: JSONObject,
    ): ChatGptWebNativeVoiceTranscriptSpeaker? {
        val parts = content.optJSONArray("parts") ?: return null
        return (0 until minOf(parts.length(), MAX_CONTENT_PARTS)).asSequence()
            .mapNotNull(parts::optJSONObject)
            .firstOrNull { it.optString("content_type") == "audio_transcription" }
            ?.optString("direction")
            ?.let { direction ->
                when (direction) {
                    "in" -> ChatGptWebNativeVoiceTranscriptSpeaker.USER
                    "out" -> ChatGptWebNativeVoiceTranscriptSpeaker.ASSISTANT
                    else -> null
                }
            }
    }

    private fun messageText(content: JSONObject): String {
        val parts = content.optJSONArray("parts")
        if (parts != null) {
            return (0 until minOf(parts.length(), MAX_CONTENT_PARTS))
                .mapNotNull { index -> partText(parts.opt(index)) }
                .joinToString(separator = "\n\n")
                .take(MAX_TRANSCRIPT_CHARS)
        }
        return DIRECT_TEXT_FIELDS.asSequence()
            .mapNotNull { field -> content.opt(field) as? String }
            .firstOrNull()
            .orEmpty()
            .take(MAX_TRANSCRIPT_CHARS)
    }

    private fun partText(value: Any?): String? = when (value) {
        is String -> value.takeIf { it.length <= MAX_TRANSCRIPT_CHARS }
        is JSONObject -> DIRECT_TEXT_FIELDS.asSequence()
            .mapNotNull { field -> value.opt(field) as? String }
            .firstOrNull { it.length <= MAX_TRANSCRIPT_CHARS }
        else -> null
    }

    private fun eventId(event: JSONObject): String? = sequenceOf(
        event.optString("event_id"),
        event.optJSONObject("payload")?.optString("event_id").orEmpty(),
    ).firstOrNull(ChatGptWebNativeVoiceTranscriptParser::validIdentifier)

    private fun trimStreams() {
        while (previousTextByStream.size > MAX_TRANSCRIPT_STREAMS) {
            previousTextByStream.remove(previousTextByStream.keys.first())
        }
    }

    private companion object {
        const val TYPE_CHAT_MESSAGE_DELTA = "chat_message_delta"
        const val TYPE_FULL_CHAT_MESSAGE = "full_chat_message"
        const val TYPE_USER_TRANSCRIPTION = "user_transcription_text"
        const val TYPE_LIVE_CAPTION = "live_captioning_text"
        val DIRECT_TEXT_FIELDS = listOf("text", "transcript", "caption")
        val TERMINAL_MESSAGE_STATES = setOf(
            "completed",
            "finished",
            "finished_successfully",
            "done",
        )
        const val MAX_TRANSCRIPT_CHARS = 64 * 1024
        const val MAX_TRANSCRIPT_STREAMS = 32
        const val MAX_CONTENT_PARTS = 32
    }
}
