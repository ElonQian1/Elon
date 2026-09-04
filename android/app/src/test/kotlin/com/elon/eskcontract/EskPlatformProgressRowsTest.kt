package com.elon.eskcontract

import com.elon.eskcontract.EskPlatformProgressFixtures.DIGEST
import com.elon.eskcontract.EskPlatformProgressFixtures.accepts
import com.elon.eskcontract.EskPlatformProgressFixtures.cursor
import com.elon.eskcontract.EskPlatformProgressFixtures.id
import com.elon.eskcontract.EskPlatformProgressFixtures.page
import com.elon.eskcontract.EskPlatformProgressFixtures.row
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Exact single-page vectors; no concatenated history or real user records. */
class EskPlatformProgressRowsTest {
    @Test fun completeFirstMiddleAndLastPagesAreIndependentlyValid() {
        assertTrue(accepts(page()))
        val first = page(count = 5, open = 2, reserved = 4)
        assertEquals("true", first["has_more"])
        assertEquals(cursor(1), first["next_cursor"])
        assertTrue(accepts(first))
        val middle = page(count = 5, open = 2, reserved = 4, start = 3, requestedCursor = cursor(3))
        assertTrue(accepts(middle, cursor(3)))
        val last = page(count = 5, open = 2, reserved = 4, start = 4, requestedCursor = cursor(3))
        assertEquals("false", last["has_more"])
        assertEquals("", last["next_cursor"])
        assertTrue(accepts(last, cursor(3)))
    }

    @Test fun twentyRowsFitMaximumKeyBudgetAndRemainExact() {
        val rows = (20 downTo 1).map { row(it) }
        val fields = page(rows, total = 20, reserved = 20, open = 20)
        assertEquals(155, fields.size)
        assertTrue(accepts(fields))
        assertFalse(accepts(fields + ("request_20_id" to id(0))))
    }

    @Test fun emptyRequestLedgerDoesNotFabricateZeroTotal() {
        val empty = page(emptyList(), reserved = 0, open = 0)
        assertTrue(accepts(empty)) // A valid total may exist without any sellback requests.
        assertTrue(accepts(page(emptyList(), total = 0, reserved = 0, open = 0)))
        assertFalse(accepts(empty + ("request_count" to "1")))
        assertFalse(accepts(empty + ("range_start" to "1")))
        assertFalse(accepts(empty + ("range_end" to "1")))
        assertFalse(accepts(empty + ("has_more" to "true")))
        assertFalse(accepts(empty + ("next_cursor" to cursor(1))))
        assertFalse(accepts(empty + ("requested_cursor" to cursor(1)), cursor(1)))
        assertFalse(accepts(empty + mapOf("reserved" to "0.000001", "reserved_base_units" to "1",
            "available" to "0.000009", "available_base_units" to "9")))
    }

    @Test fun allCanceledHistoryCanHaveNoAvailableOrReservedAssets() {
        assertTrue(accepts(page(listOf(row(2, 3, true), row(1, 7, true)), total = 0, reserved = 0, open = 0)))
        assertFalse(accepts(page(listOf(row(2, 3, true), row(1, 7, true)), total = 0, reserved = 0, open = 1)))
        assertFalse(accepts(page(listOf(row(2, 3), row(1, 7, true)), total = 0, reserved = 0, open = 0)))
    }

    @Test fun pageRangeArithmeticAvoidsOverflowAndRequiresNonemptyActualPages() {
        for ((key, value) in listOf("range_start" to "0", "range_start" to "2", "range_end" to "0",
            "range_end" to "1", "range_end" to "3", "request_count" to "1", "request_count" to "0")) {
            assertFalse(key, accepts(page() + (key to value)))
        }
        val edge = page(listOf(row(1)), total = 1, reserved = 1, count = Long.MAX_VALUE,
            start = Long.MAX_VALUE, requestedCursor = cursor(2))
        assertTrue(accepts(edge, cursor(2)))
        val overflow = page(count = Long.MAX_VALUE, start = 3, requestedCursor = cursor(3)) +
            mapOf("range_start" to Long.MAX_VALUE.toString(), "range_end" to "0", "has_more" to "false", "next_cursor" to "")
        assertFalse(accepts(overflow, cursor(3)))
    }

    @Test fun continuationBindsSnapshotAnchorAndRangeWithoutFallback() {
        val fields = page(count = 5, open = 2, reserved = 4, start = 3, requestedCursor = cursor(3))
        assertTrue(accepts(fields, cursor(3)))
        assertFalse(accepts(fields, cursor(4)))
        assertFalse(accepts(fields + ("snapshot_digest" to "c".repeat(64)), cursor(3)))
        assertFalse(accepts(fields + ("request_0_id" to id(3)), cursor(3)))
        assertFalse(accepts(fields + mapOf("range_start" to "1", "range_end" to "2"), cursor(3)))
        assertFalse(accepts(fields + ("requested_cursor" to "")))
        assertFalse(accepts(fields + ("requested_cursor" to ""), cursor(3)))
    }

    @Test fun hasMoreAndNextCursorBindActualLastRowAndExactDigest() {
        val first = page(count = 5, open = 2, reserved = 4)
        for (bad in listOf("false", "True", "1", "")) assertFalse(accepts(first + ("has_more" to bad)))
        for (bad in listOf("", cursor(2), "esbr1.${"c".repeat(64)}.${id(1)}", cursor(1) + "\n")) {
            assertFalse(accepts(first + ("next_cursor" to bad)))
        }
        assertFalse(accepts(page() + ("has_more" to "true")))
        assertFalse(accepts(page() + ("next_cursor" to cursor(1))))
        val middle = page(count = 5, open = 2, reserved = 4, start = 3, requestedCursor = cursor(3))
        assertFalse(accepts(middle + ("next_cursor" to cursor(3)), cursor(3)))
    }

    @Test fun pageIdentifiersAreExactUniqueAndNeverDisclosureExtensions() {
        for (bad in listOf("", "eskpsr_" + "a".repeat(31), "eskpsr_" + "A".repeat(32),
            "eskpsc_" + "a".repeat(32), id(2) + "\n", " $DIGEST", id(1))) {
            assertFalse(bad, accepts(page() + ("request_0_id" to bad)))
        }
        assertFalse(accepts(page() + ("request_0_amount" to "0.000000") + ("request_0_amount_base_units" to "0")))
        assertFalse(accepts(page() + ("request_0_amount_base_units" to "4")))
        assertFalse(accepts(page() + ("request_0_amount" to "0.000004")))
    }

    @Test fun submissionAndCancellationDoNotImplySettlement() {
        for (bad in listOf("pending", "settled", "paid", "Submitted", "", "submitted\n")) {
            assertFalse(accepts(page() + ("request_0_status" to bad)))
        }
        assertFalse(accepts(page() + ("request_0_canceled_at" to "2026-09-04T12:00:00Z")))
        assertFalse(accepts(page() + ("request_1_canceled_at" to "")))
        assertFalse(accepts(page() + ("request_1_canceled_at" to "2026-09-04T11:59:59.999999999Z")))
        assertTrue(accepts(page() + ("request_1_canceled_at" to "2026-09-04T12:00:00.000000000+00:00")))
    }

    @Test fun utcDatesRejectInvalidCalendarLeapSecondZonesAndPrecision() {
        for (bad in listOf("2026-02-29T12:00:00Z", "2026-04-31T12:00:00Z", "2026-09-04T24:00:00Z",
            "2026-09-04T12:00:60Z", "2026-09-04T12:00:00+01:00", "2026-09-04T12:00:00-00:00",
            "2026-09-04T12:00:00", "2026-09-04 12:00:00Z", "2026-9-04T12:00:00Z",
            "2026-09-04T12:00:00.1234567890Z", "2026-09-04T12:00:00.Z", "2026-09-04T12:00:00Z\n",
            "+2026-09-04T12:00:00Z", "2026-09-04t12:00:00z", "2026-09-04T12:00:00+00:00:00")) {
            assertFalse(bad, accepts(page() + ("request_0_created_at" to bad)))
            assertFalse(bad, accepts(page() + ("request_1_canceled_at" to bad)))
        }
        assertTrue(accepts(page(listOf(row(2, 3, created = "2024-02-29T12:00:00Z"),
            row(1, 7, true, "2024-02-29T11:00:00Z")))))
    }

    @Test fun orderingUsesParsedUtcNanosecondsThenIdNotRawTimeText() {
        val equalInstants = page(listOf(row(2, 3, created = "2026-09-04T12:00:00+00:00"),
            row(1, 7, true, "2026-09-04T12:00:00.000000000Z")))
        assertTrue(accepts(equalInstants))
        assertFalse(accepts(equalInstants + ("request_0_id" to id(0))))
        assertFalse(accepts(page() + ("request_0_created_at" to "2026-09-04T11:59:59.999999999Z")))
        val nanos = page(listOf(row(1, 3, created = "2026-09-04T12:00:00.000000002Z"),
            row(2, 7, true, "2026-09-04T12:00:00.000000001+00:00")))
        assertTrue(accepts(nanos)) // Earlier row ID can be smaller when its timestamp is newer.
        assertFalse(accepts(nanos + ("request_1_created_at" to "2026-09-04T12:00:00.000000003Z")))
    }

    @Test fun fullPageCountsAndOpenAmountsMustEqualGlobalValues() {
        assertFalse(accepts(page(open = 2)))
        assertFalse(accepts(page(reserved = 4)))
        assertFalse(accepts(page(reserved = 2)))
        assertFalse(accepts(page(open = 0)))
        assertTrue(accepts(page(listOf(row(2, 3), row(1, 7)), total = 10, reserved = 10, open = 2)))
    }

    @Test fun unseenOpenRowsHaveCountCapacityAndAtLeastOneMicroUnitEach() {
        // Two unseen requests, both open: their minimum amount is exactly two micro units.
        assertTrue(accepts(page(count = 4, open = 3, reserved = 5)))
        assertFalse(accepts(page(count = 4, open = 3, reserved = 4)))
        assertFalse(accepts(page(count = 4, open = 4, reserved = 6)))
        assertTrue(accepts(page(count = 4, open = 1, reserved = 3)))
        assertFalse(accepts(page(count = 4, open = 1, reserved = 4)))
        // A partial page with no open row still needs nonzero reserve if unseen rows are open.
        val canceledOnly = listOf(row(2, 3, true), row(1, 7, true))
        assertTrue(accepts(page(canceledOnly, count = 4, open = 2, reserved = 2)))
        assertFalse(accepts(page(canceledOnly, count = 4, open = 2, reserved = 1)))
    }

    @Test fun submittedAmountOverflowFailsWithoutWrappingIntoAvailableFunds() {
        val overflow = page(listOf(row(2, Long.MAX_VALUE), row(1)), total = Long.MAX_VALUE,
            reserved = Long.MAX_VALUE, open = 2)
        assertFalse(accepts(overflow))
        val maximum = page(listOf(row(2, Long.MAX_VALUE - 1), row(1)), total = Long.MAX_VALUE,
            reserved = Long.MAX_VALUE, open = 2)
        assertTrue(accepts(maximum))
        // Canceled amounts are history, not a sum to be presented as reserve or current balance.
        assertTrue(accepts(page(listOf(row(2, Long.MAX_VALUE, true), row(1, Long.MAX_VALUE, true)),
            total = 0, reserved = 0, open = 0)))
    }
}
