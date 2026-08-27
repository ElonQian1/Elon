package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatSendCommandLedgerTest {
    @Test
    fun beginCreatesStableRequestIdAndRejectsSecondActiveCommand() {
        val ledger = WebChatSendCommandLedger()

        val command = requireNotNull(ledger.begin("hello", WebChatSendAuthority.OFFICIAL_PAGE))

        assertTrue(command.id.matches(Regex("mcp_[a-z0-9]{1,32}")))
        assertEquals(command.id, ledger.commandId())
        assertNull(ledger.begin("again", WebChatSendAuthority.OFFICIAL_PAGE))
        assertEquals(
            WebChatSendCommandLedger.FallbackDecision.SAFE_BEFORE_DISPATCH,
            ledger.fallbackDecision(),
        )
    }

    @Test
    fun queuedCommandRequiresReconciliationUntilMatchingReceiptArrives() {
        val ledger = WebChatSendCommandLedger()
        val command = requireNotNull(ledger.begin("hello", WebChatSendAuthority.OFFICIAL_PAGE))

        assertTrue(ledger.markDispatched(command.id))
        assertEquals(
            WebChatSendCommandLedger.FallbackDecision.RECONCILE_ONLY,
            ledger.fallbackDecision(),
        )
        assertEquals(
            WebChatSendCommandLedger.ReceiptResult.IGNORED,
            ledger.acceptReceipt("mcp_stale", ok = true),
        )
        assertEquals(WebChatSendAcceptance.DISPATCHED_UNCONFIRMED, ledger.current()?.acceptance)

        assertEquals(
            WebChatSendCommandLedger.ReceiptResult.ACCEPTED,
            ledger.acceptReceipt(command.id, ok = true),
        )
        assertEquals(WebChatSendAcceptance.ACCEPTED, ledger.current()?.acceptance)
        assertEquals(WebChatPageSyncState.CLEAN, ledger.current()?.pageSyncState)
        assertEquals(
            WebChatSendCommandLedger.FallbackDecision.FORBIDDEN_AFTER_ACCEPTANCE,
            ledger.fallbackDecision(),
        )
    }

    @Test
    fun privateAcceptanceMarksOfficialPageDirtyUntilReconciled() {
        val ledger = WebChatSendCommandLedger()
        val command = requireNotNull(
            ledger.begin("hello", WebChatSendAuthority.SAME_ORIGIN_PRIVATE),
        )
        ledger.markDispatched(command.id)

        ledger.acceptReceipt(command.id, ok = true)

        assertEquals(WebChatPageSyncState.DIRTY, ledger.current()?.pageSyncState)
        assertTrue(ledger.markPageReconciliationStarted(command.id))
        assertEquals(WebChatPageSyncState.RECONCILING, ledger.current()?.pageSyncState)
        assertTrue(ledger.markPageReconciled(command.id))
        assertEquals(WebChatPageSyncState.CLEAN, ledger.current()?.pageSyncState)
    }

    @Test
    fun completedTurnArchivesSettledCommandAndAllowsNextCommand() {
        val ledger = WebChatSendCommandLedger()
        val command = requireNotNull(ledger.begin("hello world", WebChatSendAuthority.OFFICIAL_PAGE))
        ledger.markDispatched(command.id)
        ledger.acceptReceipt(command.id, ok = true)

        assertTrue(ledger.observeCompletedTurn(" hello   world ", assistantObserved = true))
        assertNull(ledger.current())
        assertEquals(WebChatSendAcceptance.SETTLED, ledger.history().single().acceptance)
        assertTrue(ledger.begin("next", WebChatSendAuthority.OFFICIAL_PAGE) != null)
    }

    @Test
    fun failedReceiptArchivesCommandAndReturnsNoActiveFallback() {
        val ledger = WebChatSendCommandLedger()
        val command = requireNotNull(ledger.begin("retry", WebChatSendAuthority.OFFICIAL_PAGE))
        ledger.markDispatched(command.id)

        assertEquals(
            WebChatSendCommandLedger.ReceiptResult.FAILED,
            ledger.acceptReceipt(command.id, ok = false),
        )
        assertNull(ledger.current())
        assertEquals(WebChatSendAcceptance.FAILED, ledger.history().single().acceptance)
        assertEquals(
            WebChatSendCommandLedger.FallbackDecision.NOT_APPLICABLE,
            ledger.fallbackDecision(),
        )
        assertFalse(ledger.markPageReconciled(command.id))
    }

    @Test
    fun missingReceiptBecomesUnknownAndNeverLooksSafeToReplay() {
        val ledger = WebChatSendCommandLedger()
        val command = requireNotNull(ledger.begin("maybe sent", WebChatSendAuthority.OFFICIAL_PAGE))
        ledger.markDispatched(command.id)

        assertEquals(
            WebChatPendingSendState.TimeoutAction.KEEP_WAITING,
            ledger.onConfirmationTimeout(command.generation).action,
        )
        assertEquals(WebChatSendAcceptance.UNKNOWN, ledger.current()?.acceptance)
        assertEquals(WebChatPageSyncState.RECONCILING, ledger.current()?.pageSyncState)
        assertEquals(
            WebChatSendCommandLedger.FallbackDecision.RECONCILE_ONLY,
            ledger.fallbackDecision(),
        )

        ledger.onConfirmationTimeout(command.generation)
        assertEquals(
            WebChatPendingSendState.TimeoutAction.REQUIRE_RECONCILIATION,
            ledger.onConfirmationTimeout(command.generation).action,
        )
        assertEquals("maybe sent", ledger.prompt())
    }
}
