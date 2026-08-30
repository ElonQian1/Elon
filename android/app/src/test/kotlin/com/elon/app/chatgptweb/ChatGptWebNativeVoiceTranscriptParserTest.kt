package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebNativeVoiceTranscriptParserTest {
    @Test
    fun parsesGaAssistantTranscriptDelta() {
        val event = parse(
            type = "response.output_audio_transcript.delta",
            itemId = "item_assistant_1",
            textKey = "delta",
            text = "你",
        )

        assertEquals(ChatGptWebNativeVoiceTranscriptSpeaker.ASSISTANT, event?.speaker)
        assertEquals(ChatGptWebNativeVoiceTranscriptUpdate.DELTA, event?.update)
        assertEquals("item_assistant_1", event?.streamKey)
        assertEquals("你", event?.text)
    }

    @Test
    fun parsesCompletedUserTranscription() {
        val event = parse(
            type = "conversation.item.input_audio_transcription.completed",
            itemId = "item_user_1",
            textKey = "transcript",
            text = "你好",
        )

        assertEquals(ChatGptWebNativeVoiceTranscriptSpeaker.USER, event?.speaker)
        assertEquals(ChatGptWebNativeVoiceTranscriptUpdate.FINAL, event?.update)
        assertEquals("你好", event?.text)
    }

    @Test
    fun acceptsLegacyAssistantTranscriptEventsDuringProviderMigration() {
        val event = parse(
            type = "response.audio_transcript.done",
            itemId = "item_assistant_legacy",
            textKey = "transcript",
            text = "兼容完成字幕",
        )

        assertEquals(ChatGptWebNativeVoiceTranscriptSpeaker.ASSISTANT, event?.speaker)
        assertEquals(ChatGptWebNativeVoiceTranscriptUpdate.FINAL, event?.update)
    }

    @Test
    fun unwrapsPrivateDataMessageObjectAndStringEnvelopes() {
        val transcript = JSONObject()
            .put("type", "response.output_audio_transcript.done")
            .put("item_id", "item_wrapped")
            .put("transcript", "完成")
        val objectEnvelope = JSONObject()
            .put("type", "data_message")
            .put("data", transcript)
        val stringEnvelope = JSONObject()
            .put("type", "data_message")
            .put("payload", transcript.toString())

        assertEquals(
            "完成",
            ChatGptWebNativeVoiceTranscriptParser.parse(objectEnvelope.toString())?.text,
        )
        assertEquals(
            "完成",
            ChatGptWebNativeVoiceTranscriptParser.parse(stringEnvelope.toString())?.text,
        )
        assertEquals(
            "response.output_audio_transcript.done",
            ChatGptWebNativeVoiceTranscriptParser.structuralEventType(stringEnvelope.toString()),
        )
    }

    @Test
    fun rejectsUnrelatedEventsMissingStableItemBindingAndOversizedPayloads() {
        assertNull(
            ChatGptWebNativeVoiceTranscriptParser.parse(
                JSONObject()
                    .put("type", "response.output_audio.delta")
                    .put("item_id", "item_1")
                    .put("delta", "audio")
                    .toString(),
            ),
        )
        assertNull(
            ChatGptWebNativeVoiceTranscriptParser.parse(
                JSONObject()
                    .put("type", "response.output_audio_transcript.delta")
                    .put("delta", "missing item")
                    .toString(),
            ),
        )
        assertNull(ChatGptWebNativeVoiceTranscriptParser.parse("x".repeat(256 * 1024 + 1)))
    }

    @Test
    fun reconstructsPrivateChatMessageDeltaIntoIncrementalNativeCaptions() {
        val decoder = ChatGptWebNativeVoiceTranscriptDecoder()
        val first = decoder.decode(
            privateEvent(
                "chat_message_delta",
                JSONObject()
                    .put("c", 0)
                    .put("o", "add")
                    .put("p", "")
                    .put("v", JSONObject().put("message", privateMessage("你", "in_progress"))),
            ),
        )
        val second = decoder.decode(
            privateEvent(
                "chat_message_delta",
                JSONObject()
                    .put("o", "append")
                    .put("p", "/message/content/parts/0")
                    .put("v", "好"),
            ),
        )
        val finished = decoder.decode(
            privateEvent(
                "chat_message_delta",
                JSONObject()
                    .put("o", "replace")
                    .put("p", "/message/status")
                    .put("v", "finished_successfully"),
            ),
        )

        assertEquals(ChatGptWebNativeVoiceTranscriptSpeaker.ASSISTANT, first?.speaker)
        assertEquals(ChatGptWebNativeVoiceTranscriptUpdate.DELTA, first?.update)
        assertEquals("你", first?.text)
        assertEquals("好", second?.text)
        assertEquals(ChatGptWebNativeVoiceTranscriptUpdate.FINAL, finished?.update)
        assertEquals("你好", finished?.text)
    }

    @Test
    fun parsesPrivateFullUserAudioTranscriptionAndAssistantMessage() {
        val decoder = ChatGptWebNativeVoiceTranscriptDecoder()
        val userMessage = JSONObject()
            .put("id", "voice_user_1")
            .put("author", JSONObject().put("role", "user"))
            .put("status", "finished_successfully")
            .put(
                "content",
                JSONObject()
                    .put("content_type", "multimodal_text")
                    .put(
                        "parts",
                        JSONArray().put(
                            JSONObject()
                                .put("content_type", "audio_transcription")
                                .put("direction", "in")
                                .put("text", "语音问题"),
                        ),
                    ),
            )
        val user = decoder.decode(fullMessageEvent(userMessage))
        val assistant = decoder.decode(
            fullMessageEvent(privateMessage("语音回答", "finished_successfully")),
        )

        assertEquals(ChatGptWebNativeVoiceTranscriptSpeaker.USER, user?.speaker)
        assertEquals("语音问题", user?.text)
        assertEquals(ChatGptWebNativeVoiceTranscriptSpeaker.ASSISTANT, assistant?.speaker)
        assertEquals("语音回答", assistant?.text)
        assertEquals(ChatGptWebNativeVoiceTranscriptUpdate.FINAL, assistant?.update)
    }

    @Test
    fun ignoresPrivateMetricsAndUnknownNestedText() {
        val decoder = ChatGptWebNativeVoiceTranscriptDecoder()
        assertNull(
            decoder.decode(
                JSONObject()
                    .put("type", "data_message")
                    .put(
                        "data",
                        JSONObject()
                            .put("type", "client_metrics")
                            .put("payload", JSONObject().put("text", "not a caption"))
                            .toString(),
                    )
                    .toString(),
            ),
        )
        assertNull(
            decoder.decode(
                JSONObject()
                    .put("type", "data_message")
                    .put(
                        "data",
                        JSONObject()
                            .put("type", "user_transcription_text")
                            .put("payload", JSONObject().put("metadata", JSONObject().put("text", "hidden")))
                            .toString(),
                    )
                    .toString(),
            ),
        )
    }

    private fun parse(
        type: String,
        itemId: String,
        textKey: String,
        text: String,
    ): ChatGptWebNativeVoiceTranscriptEvent? =
        ChatGptWebNativeVoiceTranscriptParser.parse(
            JSONObject()
                .put("event_id", "event_1")
                .put("type", type)
                .put("item_id", itemId)
                .put(textKey, text)
                .toString(),
        )

    private fun privateEvent(type: String, delta: JSONObject): String = JSONObject()
        .put("type", "data_message")
        .put(
            "data",
            JSONObject()
                .put("type", type)
                .put("payload", JSONObject().put("type", type).put("delta", delta))
                .toString(),
        )
        .toString()

    private fun fullMessageEvent(message: JSONObject): String = JSONObject()
        .put("type", "data_message")
        .put(
            "data",
            JSONObject()
                .put("type", "full_chat_message")
                .put(
                    "payload",
                    JSONObject()
                        .put("type", "full_chat_message")
                        .put("message", message),
                )
                .toString(),
        )
        .toString()

    private fun privateMessage(text: String, status: String): JSONObject = JSONObject()
        .put("id", "voice_assistant_1")
        .put("author", JSONObject().put("role", "assistant"))
        .put("status", status)
        .put("content", JSONObject().put("content_type", "text").put("parts", JSONArray().put(text)))
}
