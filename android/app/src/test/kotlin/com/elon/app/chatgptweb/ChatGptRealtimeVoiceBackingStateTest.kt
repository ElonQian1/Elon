package com.elon.app.chatgptweb

import com.elon.app.WebChatManagedRealtimeVoicePhase
import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptRealtimeVoiceBackingStateTest {
    @Test
    fun connectedTransportIsActiveBeforeMediaObservationsArrive() {
        val state = ChatGptWebNativeVoiceState(
            phase = ChatGptWebNativeVoicePhase.CONNECTED,
            remoteAudio = false,
            dataChannelOpen = false,
        ).toManagedRealtimeVoiceState(enabled = true)

        assertEquals(WebChatManagedRealtimeVoicePhase.ACTIVE, state.phase)
        assertEquals("media_observation_pending", state.code)
    }

    @Test
    fun completeConnectedTransportRemainsActiveWithoutPendingDetail() {
        val state = ChatGptWebNativeVoiceState(
            phase = ChatGptWebNativeVoicePhase.CONNECTED,
            remoteAudio = true,
            dataChannelOpen = true,
        ).toManagedRealtimeVoiceState(enabled = true)

        assertEquals(WebChatManagedRealtimeVoicePhase.ACTIVE, state.phase)
        assertEquals(null, state.code)
    }

    @Test
    fun disabledTransportRemainsUnavailable() {
        val state = ChatGptWebNativeVoiceState(
            phase = ChatGptWebNativeVoicePhase.CONNECTED,
        ).toManagedRealtimeVoiceState(enabled = false)

        assertEquals(WebChatManagedRealtimeVoicePhase.UNAVAILABLE, state.phase)
    }
}
