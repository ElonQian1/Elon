package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebRealtimeVoiceResearchObservationTest {
    @Test
    fun acceptsBoundedShapeOnlyObservations() {
        val value = "v1|network-end|post|chatgpt-origin|/backend-api/realtime/{id}|200|json|b2"
        val observation = requireNotNull(
            ChatGptWebRealtimeVoiceResearchObservation.parse(value),
        )

        assertEquals("network-end", observation.channel)
        assertEquals(value, observation.detail)
        assertEquals(value, observation.traceDetails()["summary"])
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
