package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatRealtimeVoiceStateTest {
    @Test
    fun activeVoiceDefaultsToIdleUntilAnOfficialTurnSignalIsObserved() {
        val state = WebChatRealtimeVoiceState(
            lifecycle = WebChatRealtimeVoiceLifecycle.ACTIVE,
            detail = "connected",
        )

        assertEquals(
            WebChatRealtimeVoiceVisibleState.IDLE,
            WebChatRealtimeVoiceStatePolicy.visibleState(state),
        )
    }

    @Test
    fun verifiedTurnSignalsOverrideTheActiveIdlePresentation() {
        val visibleStates = listOf(
            WebChatRealtimeVoiceTurn.LISTENING to WebChatRealtimeVoiceVisibleState.LISTENING,
            WebChatRealtimeVoiceTurn.THINKING to WebChatRealtimeVoiceVisibleState.THINKING,
            WebChatRealtimeVoiceTurn.SPEAKING to WebChatRealtimeVoiceVisibleState.SPEAKING,
        )

        visibleStates.forEach { (turn, expected) ->
            assertEquals(
                expected,
                WebChatRealtimeVoiceStatePolicy.visibleState(
                    WebChatRealtimeVoiceState(
                        lifecycle = WebChatRealtimeVoiceLifecycle.ACTIVE,
                        detail = "active",
                        turn = turn,
                    ),
                ),
            )
        }
    }

    @Test
    fun endingCannotBeConfusedWithConnecting() {
        val state = WebChatRealtimeVoiceState(
            lifecycle = WebChatRealtimeVoiceLifecycle.ENDING,
            detail = "ending",
        )

        assertEquals(
            WebChatRealtimeVoiceVisibleState.ENDING,
            WebChatRealtimeVoiceStatePolicy.visibleState(state),
        )
    }
}
