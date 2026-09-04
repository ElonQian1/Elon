package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebResearchResourceShapeTest {
    @Test
    fun recordsMutatingChatGptRequestWithoutQueryOrIdentifier() {
        assertEquals(
            "v1|resource-start|post|chatgpt|/backend-api/speech/{id}|audio",
            ChatGptWebResearchResourceShape.from(
                method = "POST",
                rawUrl = "https://chatgpt.com/backend-api/speech/12345678901234567?proof=private",
                contentType = "audio/webm; codecs=opus",
            ),
        )
    }

    @Test
    fun ignoresTelemetryAndUnrelatedHosts() {
        assertNull(
            ChatGptWebResearchResourceShape.from(
                method = "POST",
                rawUrl = "https://chatgpt.com/ces/v1/telemetry/intake",
                contentType = "application/json",
            ),
        )
        assertNull(
            ChatGptWebResearchResourceShape.from(
                method = "POST",
                rawUrl = "https://example.com/audio/upload",
                contentType = "audio/webm",
            ),
        )
    }

    @Test
    fun onlyKeepsVoiceRelatedGetRequests() {
        assertEquals(
            "v1|resource-start|get|openai|/voice/config|none",
            ChatGptWebResearchResourceShape.from(
                method = "GET",
                rawUrl = "https://api.openai.com/voice/config",
                contentType = null,
            ),
        )
        assertNull(
            ChatGptWebResearchResourceShape.from(
                method = "GET",
                rawUrl = "https://chatgpt.com/backend-api/conversations",
                contentType = "application/json",
            ),
        )
        assertEquals(
            "v1|resource-start|get|chatgpt|/backend-api/synthesize|none",
            ChatGptWebResearchResourceShape.from(
                method = "GET",
                rawUrl = "https://chatgpt.com/backend-api/synthesize?message_id=private",
                contentType = null,
            ),
        )
    }
}
