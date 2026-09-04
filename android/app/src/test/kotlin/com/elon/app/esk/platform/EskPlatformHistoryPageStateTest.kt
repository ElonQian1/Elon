package com.elon.app.esk.platform

import org.junit.Assert.*
import org.junit.Test

class EskPlatformHistoryPageStateTest {
    private fun session(change: (MutableMap<String, Any?>) -> Unit = {}) = requireNotNull(
        EskPlatformSession.fromPreferences(eskPlatformSessionValues().apply(change), 1_000L))

    private fun page(index: Int = 1): EskPlatformHistoryPage {
        val entry = EskPlatformEntry("eskp_entry_${(4 - index).toString().padStart(32, '0')}",
            "eskp_allocation_${index.toString().padStart(32, '0')}", "1.000000", "1000000", "2026-09-04T00:00:00Z")
        return EskPlatformHistoryPage(total = "3.000000", totalBaseUnits = "3000000", entryCount = "3",
            updatedAt = entry.createdAt, snapshotDigest = "a".repeat(64), rangeStart = "$index", rangeEnd = "$index",
            hasMore = index < 3, nextCursor = if (index < 3) "ephp1.${"a".repeat(64)}.${entry.entryId}" else null,
            entries = listOf(entry))
    }

    private fun loaded(): EskPlatformHistoryPageState = EskPlatformHistoryPageState().apply {
        assertTrue(accept(first(session()), page(), session(), 100L, 1_000L, true))
    }

    @Test fun currentPagesContinueWithoutAccumulatingEarlierRecords() {
        val state = loaded()
        for (index in 2..3) {
            val ticket = requireNotNull(state.next(session(), 100L + index * 20L, 1_000L, true))
            assertEquals(page(index - 1).nextCursor, ticket.cursor)
            assertEquals("EskPlatformHistoryTicket(redacted)", ticket.toString())
            assertTrue(state.accept(ticket, page(index), session(), 110L + index * 20L, 1_000L, true))
            assertFalse(state.accept(ticket, page(index), session(), 110L + index * 20L, 1_000L, true))
        }
        assertNull(state.next(session(), 120L, 1_000L, true))
    }

    @Test fun emptyAccountIsAValidFirstPageOnly() {
        val state = EskPlatformHistoryPageState()
        val empty = page().copy(total = "0.000000", totalBaseUnits = "0", entryCount = "0", updatedAt = null,
            rangeStart = "0", rangeEnd = "0", entries = emptyList(), hasMore = false, nextCursor = null)
        assertTrue(state.accept(state.first(session()), empty, session(), 100L, 1_000L, true))
        assertNull(state.next(session(), 101L, 1_000L, true))
        val nextState = loaded()
        val ticket = requireNotNull(nextState.next(session(), 101L, 1_000L, true))
        assertFalse(nextState.accept(ticket, empty, session(), 102L, 1_000L, true))
    }

    @Test fun lateCallbackDoesNotInvalidateNewerFirstPageRequest() {
        val state = EskPlatformHistoryPageState()
        val old = state.first(session())
        val latest = state.first(session())
        assertFalse(state.accept(old, page(), session(), 100L, 1_000L, true))
        assertTrue(state.accept(latest, page(), session(), 101L, 1_000L, true))
    }

    @Test fun clearingForBackgroundSaveOrAccountChangeInvalidatesPendingAndDisplayed() {
        val state = loaded()
        val ticket = requireNotNull(state.next(session(), 101L, 1_000L, true))
        state.clear()
        assertFalse(state.accept(ticket, page(2), session(), 102L, 1_000L, true))
        assertNull(state.next(session(), 103L, 1_000L, true))
    }

    @Test fun accountSwitchIncludingRestoredUserAndTokenCannotContinue() {
        for ((key, value) in listOf("auth_user_id" to "fixture-b", "auth_token" to "fixture-b-token",
            "auth_session_revision" to "00000000-0000-4000-8000-000000000003")) {
            val state = loaded()
            assertNull(state.next(session { it[key] = value }, 101L, 1_000L, true))
            assertNull(state.next(session(), 102L, 1_000L, true))
        }
    }

    @Test fun continuationMustStillBeVisibleFreshAndAuthenticated() {
        data class Case(val elapsed: Long, val epoch: Long, val foreground: Boolean)
        for (case in listOf(Case(99L, 1_000L, true), Case(60_100L, 1_000L, true),
            Case(Long.MAX_VALUE, 1_000L, true), Case(101L, 100_000L, true),
            Case(101L, -1L, true), Case(101L, 1_000L, false))) {
            assertNull(loaded().next(session(), case.elapsed, case.epoch, case.foreground))
        }
        assertNull(loaded().next(null, 101L, 1_000L, true))
    }

    @Test fun requestFinishingAfterPriorPageExpiresCannotExtendThatContext() {
        val state = loaded()
        val ticket = requireNotNull(state.next(session(), 60_099L, 1_000L, true))
        assertFalse(state.accept(ticket, page(2), session(), 60_100L, 1_000L, true))
    }

    @Test fun responseIdentityAndForegroundAreRechecked() {
        for (mode in 0..4) {
            val state = loaded()
            val ticket = requireNotNull(state.next(session(), 101L, 1_000L, true))
            val current = when (mode) { 0 -> null; 1 -> session { it["auth_token"] = "fixture-other" }; else -> session() }
            assertFalse(state.accept(ticket, page(2), current, if (mode == 2) -1L else 102L,
                if (mode == 3) 100_000L else 1_000L, mode != 4))
        }
    }

    @Test fun snapshotSummaryRangeAndCrossPageOrderCannotDrift() {
        val second = page(2)
        val variants = listOf(second.copy(snapshotDigest = "b".repeat(64)),
            second.copy(totalBaseUnits = "4000000"), second.copy(entryCount = "4"),
            second.copy(updatedAt = "2026-09-04T00:00:01Z"), second.copy(rangeStart = "3"),
            second.copy(entries = listOf(page().entries.single())),
            second.copy(entries = listOf(second.entries.single().copy(createdAt = "2026-09-04T00:00:01Z"))))
        for (candidate in variants) {
            val state = loaded()
            val ticket = requireNotNull(state.next(session(), 101L, 1_000L, true))
            assertFalse(state.accept(ticket, candidate, session(), 102L, 1_000L, true))
            assertNull(state.next(session(), 103L, 1_000L, true))
        }
    }

    @Test fun firstRequestCannotAcceptMiddleOfHistoryOrAnotherInstancesTicket() {
        val state = EskPlatformHistoryPageState()
        assertFalse(state.accept(state.first(session()), page(2), session(), 100L, 1_000L, true))
        val own = state.first(session())
        assertFalse(state.accept(EskPlatformHistoryPageState().first(session()), page(), session(), 100L, 1_000L, true))
        assertTrue(state.accept(own, page(), session(), 101L, 1_000L, true))
    }
}
