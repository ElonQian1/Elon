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
    fun staleObservationKeepsTheCallActiveWithoutShowingAnErrorCard() {
        val state = WebChatRealtimeVoiceState(
            lifecycle = WebChatRealtimeVoiceLifecycle.ACTIVE,
            detail = "syncing",
            observation = WebChatRealtimeVoiceObservation.STALE,
        )

        assertEquals(
            WebChatRealtimeVoiceVisibleState.SYNCING,
            WebChatRealtimeVoiceStatePolicy.visibleState(state),
        )
        assertEquals(
            WebChatRealtimeVoiceExpansionDecision.COLLAPSE,
            WebChatRealtimeVoiceStatePolicy.expansionDecision(
                WebChatRealtimeVoiceVisibleState.SYNCING,
            ),
        )
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

    @Test
    fun unconfirmedHangupCollapsesToTheNonBlockingOrb() {
        assertEquals(
            WebChatRealtimeVoiceExpansionDecision.COLLAPSE,
            WebChatRealtimeVoiceStatePolicy.expansionDecision(
                WebChatRealtimeVoiceVisibleState.HANGUP_UNCONFIRMED,
            ),
        )
    }

    @Test
    fun onlyARealFailureForcesTheActionCardOpen() {
        assertEquals(
            WebChatRealtimeVoiceExpansionDecision.EXPAND,
            WebChatRealtimeVoiceStatePolicy.expansionDecision(
                WebChatRealtimeVoiceVisibleState.FAILED,
            ),
        )
        assertEquals(
            WebChatRealtimeVoiceExpansionDecision.PRESERVE,
            WebChatRealtimeVoiceStatePolicy.expansionDecision(
                WebChatRealtimeVoiceVisibleState.SPEAKING,
            ),
        )
    }
}
