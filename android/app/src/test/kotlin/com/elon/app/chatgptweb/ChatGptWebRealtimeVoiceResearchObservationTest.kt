package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebRealtimeVoiceResearchObservationTest {
    @Test
    fun acceptsBoundedShapeOnlyObservations() {
        val value = "v1|network-start|post|chatgpt-origin|/realtime/wm|session-description|text|b3|offer-like"
        val observation = requireNotNull(
            ChatGptWebRealtimeVoiceResearchObservation.parse(value),
        )

        assertEquals("network-start", observation.channel)
        assertEquals(value, observation.detail)
        assertEquals(value, observation.traceDetails()["summary"])

        assertNotNull(
            ChatGptWebRealtimeVoiceResearchObservation.parse(
                "v1|network-form-shape|chatgpt-origin|/realtime/wm|json-voice.model",
            ),
        )
    }

    @Test
    fun rejectsCredentialAndSessionDescriptionMarkers() {
        listOf(
            "v1|network-shape|chatgpt-origin|/voice|client-secret",
            "v1|network-start|post|chatgpt-origin|/voice/token",
            "v1|peer-local-description|offer|sdp",
            "v1|peer-ice|candidate",
            "v1|unknown-channel|value",
            "v2|peer-created",
            "v1|peer-created|${"x".repeat(170)}",
        ).forEach { value ->
            assertNull(value, ChatGptWebRealtimeVoiceResearchObservation.parse(value))
        }
    }

    @Test
    fun traceContainsNoUnstructuredFields() {
        val observation = requireNotNull(
            ChatGptWebRealtimeVoiceResearchObservation.parse("v1|peer-connection|connected"),
        )

        assertEquals(setOf("channel", "summary"), observation.traceDetails().keys)
        assertTrue(observation.traceDetails().values.all { it.length <= 160 })
    }
}
