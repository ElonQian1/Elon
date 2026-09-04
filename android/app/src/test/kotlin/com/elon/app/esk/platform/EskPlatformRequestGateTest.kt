package com.elon.app.esk.platform

import org.junit.Assert.*
import org.junit.Test

class EskPlatformRequestGateTest {
    private fun session(change: (MutableMap<String, Any?>) -> Unit = {}): EskPlatformSession =
        requireNotNull(EskPlatformSession.fromPreferences(eskPlatformSessionValues().apply(change), 1_000L))

    @Test fun currentForegroundRequestCanBeConsumedExactlyOnce() {
        val gate = EskPlatformRequestGate()
        val initial = session()
        val ticket = requireNotNull(gate.begin(initial, 100L, 1_000L, true))
        assertTrue(gate.consume(ticket, session(), 101L, 1_001L, true))
        assertFalse(gate.consume(ticket, initial, 102L, 1_002L, true))
        assertEquals("EskPlatformRequestTicket(redacted)", ticket.toString())
    }

    @Test fun oldCallbackCannotConsumeNewRequest() {
        val gate = EskPlatformRequestGate()
        val initial = session()
        val old = requireNotNull(gate.begin(initial, 100L, 1_000L, true))
        val latest = requireNotNull(gate.begin(initial, 101L, 1_001L, true))
        assertFalse(gate.consume(old, initial, 102L, 1_002L, true))
        assertTrue(gate.consume(latest, initial, 103L, 1_003L, true))
    }

    @Test fun invalidateBlocksPausedDestroyedAndAccountChangedRequestsEvenAfterReturn() {
        val gate = EskPlatformRequestGate()
        val initial = session()
        val ticket = requireNotNull(gate.begin(initial, 100L, 1_000L, true))
        gate.invalidate()
        assertFalse(gate.consume(ticket, initial, 101L, 1_001L, true))
    }

    @Test fun eachSessionFieldIsPartOfCallbackIdentity() {
        val changes = listOf("auth_token" to "fixture-token-b", "auth_user_id" to "fixture-user-b",
            "auth_expires_at" to 200_000L, "auth_nickname" to "Other", "auth_account" to "Other account",
            "auth_session_revision" to "00000000-0000-4000-8000-000000000002")
        for ((key, value) in changes) {
            val gate = EskPlatformRequestGate()
            val ticket = requireNotNull(gate.begin(session(), 100L, 1_000L, true))
            assertFalse(key, gate.consume(ticket, session { it[key] = value }, 101L, 1_001L, true))
        }
    }

    @Test fun sameUserAndTokenRestoredAfterSwitchCannotReviveCallbackBeforePreferenceListenerRuns() {
        val gate = EskPlatformRequestGate()
        val initial = session()
        val ticket = requireNotNull(gate.begin(initial, 100L, 1_000L, true))
        val restored = session { it["auth_session_revision"] = "00000000-0000-4000-8000-000000000003" }
        assertEquals(initial.userId, restored.userId)
        assertEquals(initial.token, restored.token)
        assertFalse(gate.consume(ticket, restored, 101L, 1_001L, true))
    }

    @Test fun legacySessionCannotMatchNewlySavedSession() {
        val gate = EskPlatformRequestGate()
        val legacy = session { it.remove("auth_session_revision") }
        val ticket = requireNotNull(gate.begin(legacy, 100L, 1_000L, true))
        assertFalse(gate.consume(ticket, session(), 101L, 1_001L, true))
    }

    @Test fun expiredBackgroundMissingOrTimeInvalidResultFailsOnce() {
        data class Case(val current: EskPlatformSession?, val elapsed: Long, val epoch: Long, val foreground: Boolean)
        val initial = session()
        for (case in listOf(Case(initial, 101L, 1_001L, false), Case(null, 101L, 1_001L, true),
            Case(initial, 99L, 1_001L, true), Case(initial, 15_100L, 1_001L, true),
            Case(initial, Long.MAX_VALUE, 1_001L, true), Case(initial, 101L, 100_000L, true),
            Case(initial, 101L, -1L, true))) {
            val gate = EskPlatformRequestGate()
            val ticket = requireNotNull(gate.begin(initial, 100L, 1_000L, true))
            assertFalse(gate.consume(ticket, case.current, case.elapsed, case.epoch, case.foreground))
            assertFalse(gate.consume(ticket, initial, 101L, 1_001L, true))
        }
    }

    @Test fun lastMillisecondBeforeDeadlineAndExpiryIsAccepted() {
        val gate = EskPlatformRequestGate()
        val initial = session()
        val ticket = requireNotNull(gate.begin(initial, 100L, 1_000L, true))
        assertTrue(gate.consume(ticket, initial, 15_099L, 99_999L, true))
    }

    @Test fun invalidBeginAlsoInvalidatesPriorTicket() {
        for ((elapsed, epoch, foreground) in listOf(Triple(-1L, 1000L, true), Triple(100L, 1000L, false),
            Triple(100L, 100_000L, true), Triple(100L, -1L, true))) {
            val gate = EskPlatformRequestGate()
            val initial = session()
            val ticket = requireNotNull(gate.begin(initial, 100L, 1_000L, true))
            assertNull(gate.begin(initial, elapsed, epoch, foreground))
            assertFalse(gate.consume(ticket, initial, 101L, 1_001L, true))
        }
    }

    @Test fun sameGenerationTicketFromOtherGateCannotBeAccepted() {
        val gate = EskPlatformRequestGate()
        val other = EskPlatformRequestGate()
        val initial = session()
        val current = requireNotNull(gate.begin(initial, 100L, 1_000L, true))
        val unrelated = requireNotNull(other.begin(initial, 100L, 1_000L, true))
        assertFalse(gate.consume(unrelated, initial, 101L, 1_001L, true))
        assertTrue(gate.consume(current, initial, 101L, 1_001L, true))
    }
}
