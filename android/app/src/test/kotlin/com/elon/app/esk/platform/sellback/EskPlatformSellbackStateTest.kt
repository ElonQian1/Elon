package com.elon.app.esk.platform.sellback

import com.elon.app.esk.platform.EskPlatformSession
import org.junit.After
import org.junit.Assert.*
import org.junit.Test

class EskPlatformSellbackStateTest {
    private val f = SellbackFixture
    @After fun cleanup() { EskPlatformSellbackRecovery.clear() }
    private fun active(state: EskPlatformSellbackState, session: EskPlatformSession = f.session()): EskPlatformSellbackState.Ticket {
        val draft = requireNotNull(state.prepare(f.action(), session, 100L, 1000L, true))
        return requireNotNull(state.confirm(draft, session, 101L, 1001L, true))
    }
    @Test fun explicitConfirmationIsSingleUseAndBoundToExactSessionAndClock() {
        for (current in listOf(null, f.session(user = "other"), f.session(token = "replacement"),
            f.session(revision = "00000000-0000-0000-0000-000000000002"))) {
            val state = EskPlatformSellbackState()
            val draft = requireNotNull(state.prepare(f.action(), f.session(), 100, 1000, true))
            assertNull(state.confirm(draft, current, 101, 1001, true))
        }
        for ((now, epoch, foreground) in listOf(Triple(99L, 1001L, true), Triple(60100L, 1001L, true),
            Triple(101L, 2000000L, true), Triple(101L, 1001L, false))) {
            val state = EskPlatformSellbackState(); val session = f.session()
            val draft = requireNotNull(state.prepare(f.action(), session, 100, 1000, true))
            assertNull(state.confirm(draft, session, now, epoch, foreground))
        }
        val state = EskPlatformSellbackState(); val session = f.session()
        val draft = requireNotNull(state.prepare(f.action(), session, 100, 1000, true))
        assertNotNull(state.confirm(draft, session, 60099, 1001, true))
        assertNull(state.confirm(draft, session, 60100, 1001, true))
    }
    @Test fun unknownKeepsExactOriginalActionAndRequiresFreshConsentNotNewKey() {
        val state = EskPlatformSellbackState(); val session = f.session(); val ticket = active(state, session)
        assertTrue(state.unknown(ticket)); assertFalse(state.unknown(ticket)); assertTrue(state.unresolved())
        assertNull(state.prepare(f.action(), session, 200, 1002, true))
        val retry = requireNotNull(state.retry(session, 200, 1002, true))
        assertSame(ticket.draft.action, retry.action); assertEquals(ticket.draft.action.body, retry.action.body)
        assertEquals("fixture-key-1", retry.action.key)
        val confirmed = requireNotNull(state.confirm(retry, session, 201, 1003, true))
        assertTrue(state.complete(confirmed)); assertFalse(state.complete(confirmed)); assertFalse(state.unresolved())
    }
    @Test fun pauseOrDismissDropsPriorAuthorityAndRejectsLateResults() {
        val state = EskPlatformSellbackState(); val session = f.session()
        val draft = requireNotNull(state.prepare(f.action(), session, 100, 1000, true))
        state.dismiss(draft); assertNull(state.confirm(draft, session, 101, 1001, true))
        val ticket = active(state, session); assertTrue(state.clear())
        assertFalse(state.unknown(ticket)); assertFalse(state.complete(ticket)); assertNull(state.retry(session, 200, 1002, true))
    }
    @Test fun retryRejectsForegroundLossRevocationAndLegacyTokenReplacement() {
        val session = f.session(revision = null); val state = EskPlatformSellbackState()
        state.unknown(active(state, session))
        assertNull(state.retry(f.session(token = "replaced", revision = null), 200, 1002, true))
        assertNull(state.retry(session, 200, 1002, false)); assertNull(state.retry(session, 200, 2000000, true))
        assertNotNull(state.retry(session, 200, 1002, true))
    }
    @Test fun lookupResolvesOnlyBoundOriginalRecordAndCanceledReplayNeverReopens() {
        val state = EskPlatformSellbackState(); state.unknown(active(state))
        assertFalse(state.resolve(listOf(f.parsedRecord().copy(key = "other"))))
        assertFalse(state.resolve(listOf(f.parsedRecord().copy(amount = 2))))
        assertFalse(state.resolve(listOf(f.parsedRecord().copy(policyDigest = "d".repeat(64)))))
        assertTrue(state.resolve(listOf(f.parsedRecord(true))))
        assertFalse(state.unresolved())
        val canceled = SellbackAction.cancel(f.parsedRecord())
        assertFalse(canceled.matches(f.parsedRecord())); assertTrue(canceled.matches(f.parsedRecord(true)))
        assertFalse(canceled.matches(f.parsedRecord(true).copy(id = f.id(2))))
    }
    @Test fun exactQuotaPolicyChecksRejectZeroOverflowAndOverReservation() {
        val summary = f.parsedPage().summary
        for (amount in listOf(0L, -1L, Long.MAX_VALUE, 10000001L))
            assertThrows(IllegalArgumentException::class.java) { SellbackAction.submit(summary, amount, "key") }
        assertThrows(IllegalArgumentException::class.java) { SellbackAction.submit(summary.copy(enabled = false), 1, "key") }
        assertThrows(IllegalArgumentException::class.java) { SellbackAction.submit(summary.copy(available = 0), 1, "key") }
        assertThrows(IllegalArgumentException::class.java) { SellbackAction.submit(summary.copy(openCount = 100), 1, "key") }
        assertThrows(IllegalArgumentException::class.java) { SellbackAction.submit(summary.copy(reserved = 10000000), 1, "key") }
        assertThrows(IllegalArgumentException::class.java) { SellbackAction.submit(summary, 1, "invalid key") }
    }
    @Test fun recoveryIsReadOnlySessionBoundAndLegacyHasNoCrossBackgroundKey() {
        val session = f.session(); val action = f.action()
        EskPlatformSellbackRecovery.remember(action, session)
        val hint = requireNotNull(EskPlatformSellbackRecovery.current(session))
        assertEquals(action.key, hint.key); assertFalse(hint.cancel)
        assertFalse(EskPlatformSellbackRecovery.resolve(session, listOf(f.parsedRecord().copy(key = "other"))))
        assertTrue(EskPlatformSellbackRecovery.resolve(session, listOf(f.parsedRecord())))
        assertNull(EskPlatformSellbackRecovery.current(session))
        for (changed in listOf(f.session(user = "other"), f.session(expiry = 3000000),
            f.session(revision = "00000000-0000-0000-0000-000000000002"))) {
            EskPlatformSellbackRecovery.remember(action, session)
            assertNull(EskPlatformSellbackRecovery.current(changed)); assertNull(EskPlatformSellbackRecovery.current(session))
        }
        EskPlatformSellbackRecovery.remember(action, f.session(revision = null))
        assertNull(EskPlatformSellbackRecovery.current(f.session(revision = null)))
        assertNull(EskPlatformSellbackRecovery.current(f.session(token = "replaced", revision = null)))
    }
    @Test fun unknownCancelNeedsCanceledLookupAndWarningIsNeverWriteAuthority() {
        val session = f.session()
        EskPlatformSellbackRecovery.remember(SellbackAction.cancel(f.parsedRecord()), session)
        assertTrue(requireNotNull(EskPlatformSellbackRecovery.current(session)).cancel)
        assertFalse(EskPlatformSellbackRecovery.resolve(session, listOf(f.parsedRecord())))
        assertTrue(EskPlatformSellbackRecovery.resolve(session, listOf(f.parsedRecord(true))))
        val identity = SellbackReviewIdentity(session)
        assertTrue(identity.belongsTo(f.session()))
        assertFalse(identity.belongsTo(f.session(user = "other")))
        assertFalse(identity.belongsTo(f.session(revision = "00000000-0000-0000-0000-000000000002")))
        assertFalse(identity.belongsTo(f.session(expiry = 3000000)))
        assertFalse(identity.belongsTo(f.session(revision = null)))
        assertEquals("SellbackReviewIdentity(redacted)", identity.toString())
        val fieldNames = EskPlatformSellbackRecovery.Hint::class.java.declaredFields.map { it.name }
        for (secret in listOf("token", "body", "amount", "terms", "session")) assertFalse(secret, secret in fieldNames)
    }
    @Test fun debugRepresentationsNeverContainPrivateFields() {
        for (value in listOf(f.action(), f.parsedPage(), f.parsedPage().summary, f.parsedPage().summary.policy,
            f.parsedRecord(), SellbackReviewIdentity(f.session()))) {
            assertTrue(requireNotNull(value).toString().contains("redacted"))
            assertFalse(value.toString().contains("fixture-key")); assertFalse(value.toString().contains(f.terms))
        }
    }
}
