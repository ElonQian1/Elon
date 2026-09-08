package com.elon.app.grid.create

import org.junit.Assert.*
import org.junit.Test

class BinanceGridCreateTest {
    @Test fun secondWindowCannotOwnOrClearFirstWindowsJournalSlot() {
        val slot = BinanceCreateSlot(); val first = Any(); val second = Any()
        assertTrue(slot.acquire(first)); assertFalse(slot.acquire(second))
        slot.release(second); assertFalse(slot.acquire(second))
        slot.release(first); assertTrue(slot.acquire(second)); slot.release(second)
    }
    private fun draft(margin: String = "12.345") = BinanceGridDraft.parse(mapOf(
        "symbol" to "NEARUSDT", "direction" to "LONG", "spacing" to "ARITH", "marginType" to "ISOLATED",
        "lower" to "1.01", "upper" to "2.02", "margin" to margin, "leverage" to "3", "count" to "10",
        "autoInit" to "true", "closeOnStop" to "true"))
    @Test fun marginIsMultipliedExactlyAndNeverSubmittedAsNotional() {
        val d = draft()
        assertEquals("37.035", d.payload()["gridInitialValue"])
        assertEquals("12.345", d.margin)
        assertEquals(3, d.payload()["leverage"])
        assertFalse(d.payload().containsKey("clientStrategyId"))
        assertFalse(d.payload().containsKey("slideWindow"))
    }
    @Test fun invalidAndAmbiguousAmountsAreRejected() {
        listOf("0", "-1", "NaN", "1e2", "2,000", "2000.00001", "0.000000000000000000001").forEach {
            assertThrows(IllegalArgumentException::class.java) { draft(it) }
        }
    }
    @Test fun exactRangeComparisonDoesNotLoseTinyDifferences() {
        val values = draft().input + mapOf("lower" to "1.00000000000000000001", "upper" to "1.00000000000000000002")
        assertEquals("1.00000000000000000001", BinanceGridDraft.parse(values).payload()["gridLowerLimit"])
        assertThrows(IllegalArgumentException::class.java) { BinanceGridDraft.parse(values + ("upper" to values.getValue("lower"))) }
    }
    @Test fun undefinedOptionsAndExtraFieldsCannotReachTransport() {
        listOf("direction" to "BOTH", "autoInit" to "yes", "leverage" to "1.5", "count" to "1", "symbol" to "../NEARUSDT").forEach {
            assertThrows(IllegalArgumentException::class.java) { BinanceGridDraft.parse(draft().input + it) }
        }
        assertThrows(IllegalArgumentException::class.java) { BinanceGridDraft.parse(draft().input + ("url" to "https://example.test")) }
    }
    @Test fun preparationIsBoundToAccountDocumentAndFrozenInput() {
        var now = 100L
        val a = BinanceCreateAttempt({ now })
        a.prepare("a".repeat(64), "doc_test_123", draft(), "manual_mm_web_123456789012345678", 169)
        assertFalse(a.canSubmit("b".repeat(64), "doc_test_123"))
        assertFalse(a.canSubmit("a".repeat(64), "doc_new_123"))
        assertTrue(a.canSubmit("a".repeat(64), "doc_test_123"))
        now += 60_000
        assertFalse(a.canSubmit("a".repeat(64), "doc_test_123"))
    }
    @Test fun submissionIsConsumedBeforeNetworkAndCannotBeRepeated() {
        val a = BinanceCreateAttempt({ 100L })
        a.prepare("a".repeat(64), "doc_test_123", draft(), "manual_mm_web_123456789012345678", 169)
        a.start("a".repeat(64), "doc_test_123")
        assertEquals("submitting", a.status)
        assertThrows(IllegalArgumentException::class.java) { a.start("a".repeat(64), "doc_test_123") }
        a.unknown()
        assertFalse(a.canSubmit("a".repeat(64), "doc_test_123"))
        assertTrue(a.unresolved)
    }
    @Test fun restartCannotRestoreASendableDraft() {
        val a = BinanceCreateAttempt({ 100L })
        a.prepare("a".repeat(64), "doc_test_123", draft(), "manual_mm_web_123456789012345678", 169)
        a.start("a".repeat(64), "doc_test_123")
        val b = BinanceCreateAttempt({ 200L })
        b.restore(a.journal())
        assertEquals("unknown", b.status)
        assertNull(b.draft)
        assertFalse(b.canSubmit("a".repeat(64), "doc_test_123"))
    }
    @Test fun acceptedIsNotRunningAndLateSuccessCanResolveTimeout() {
        val a = BinanceCreateAttempt({ 100L })
        a.prepare("a".repeat(64), "doc_test_123", draft(), "manual_mm_web_123456789012345678", 169)
        a.start("a".repeat(64), "doc_test_123"); a.unknown()
        a.accepted("123", "NEW")
        assertEquals("accepted", a.status); assertEquals("NEW", a.providerStatus)
        assertTrue(a.unresolved)
        a.detail("123", "WORKING")
        assertEquals("observed", a.status); assertEquals("WORKING", a.providerStatus)
    }
}
