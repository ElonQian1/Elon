package com.elon.app.chatgptweb

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
}
