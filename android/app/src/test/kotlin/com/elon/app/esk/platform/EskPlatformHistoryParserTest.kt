package com.elon.app.esk.platform

import com.google.gson.JsonArray
import com.google.gson.JsonNull
import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test

class EskPlatformHistoryParserTest {
    private val fixture = EskPlatformHistoryFixture
    private fun parse(page: JsonObject = fixture.page(), cursor: String? = null) =
        EskPlatformHistoryParser.parse(page.toString().toByteArray(), cursor)
    private fun reject(page: JsonObject, cursor: String? = null) {
        val error = assertThrows(IllegalArgumentException::class.java) { parse(page, cursor) }
        assertEquals("ESK_PLATFORM_HISTORY_INVALID", error.message)
        assertNull(error.cause)
    }

    @Test fun firstPartialPageHasWholeAccountSummaryAndBoundedRange() {
        val page = parse()
        assertEquals("3.000000", page.total)
        assertEquals("3000000", page.totalBaseUnits)
        assertEquals("3", page.entryCount)
        assertEquals("1", page.rangeStart)
        assertEquals("2", page.rangeEnd)
        assertEquals(2, page.entries.size)
        assertEquals(fixture.TIME, page.updatedAt)
        assertTrue(page.hasMore)
        assertEquals(fixture.cursor(2), page.nextCursor)
    }

    @Test fun terminalPageDoesNotReplaceSummaryWithPageSumOrPageTime() {
        val json = fixture.page(3, 3, 3)
        json.getAsJsonArray("entries")[0].asJsonObject.addProperty("created_at", "2026-09-03T10:00:00+00:00")
        val page = parse(json, fixture.cursor(2))
        assertEquals("3.000000", page.total)
        assertEquals("1.000000", page.entries.single().amount)
        assertEquals(fixture.TIME, page.updatedAt)
        assertFalse(page.hasMore)
        assertNull(page.nextCursor)
    }

    @Test fun middlePageAcceptsGlobalNewestTimeAndMaintainsNextAnchor() {
        val page = parse(fixture.page(2, 3, 5), fixture.cursor(5))
        assertTrue(page.hasMore)
        assertEquals(fixture.cursor(3), page.nextCursor)
    }

    @Test fun emptyAccountIsExplicitAndOnlyValidForFirstRequest() {
        val page = parse(fixture.empty())
        assertEquals("0", page.rangeStart)
        assertEquals("0", page.rangeEnd)
        assertEquals("0.000000", page.total)
        assertNull(page.updatedAt)
        reject(fixture.empty(), fixture.cursor(1))
        for (key in listOf("entry_count", "range_start", "range_end", "total_base_units")) {
            reject(fixture.empty().apply { addProperty(key, "1") })
        }
        reject(fixture.empty().apply { addProperty("updated_at", fixture.TIME) })
    }

    @Test fun allTwentyRootFieldsAreRequiredAndUnknownFieldsFail() {
        val original = fixture.page()
        assertEquals(20, original.size())
        for (key in original.keySet()) reject(fixture.page().apply { remove(key) })
        for (key in listOf("user_id", "capabilities", "status_message", "history_has_more", "available")) {
            reject(fixture.page().apply { addProperty(key, "not-accepted") })
        }
    }

    @Test fun fixedFieldsNeverAcceptAlternateSourceOrCoercedTypes() {
        for (key in listOf("schema", "asset_id", "symbol", "source", "chain_status", "verification_basis")) {
            reject(fixture.page().apply { addProperty(key, "wrong") })
        }
        for (key in listOf("simulated", "funds_moved", "external_payment_verified")) {
            reject(fixture.page().apply { addProperty(key, true) })
            reject(fixture.page().apply { addProperty(key, "false") })
            reject(fixture.page().apply { addProperty(key, 0) })
        }
        reject(fixture.page().apply { addProperty("decimals", "6") })
        reject(fixture.page().apply { addProperty("decimals", 6.0) })
        reject(fixture.page().apply { addProperty("has_more", "true") })
        reject(fixture.page().apply { addProperty("next_cursor", 1) })
        reject(fixture.page().apply { addProperty("updated_at", 1) })
    }

    @Test fun everyIntegerFieldIsCanonicalNonnegativeI64String() {
        for (key in listOf("total_base_units", "entry_count", "range_start", "range_end")) {
            for (bad in listOf("", "01", "-1", "+1", "1e0", "1.0", " 1", "9223372036854775808")) {
                reject(fixture.page().apply { addProperty(key, bad) })
            }
            reject(fixture.page().apply { addProperty(key, 1) })
        }
    }

    @Test fun amountIsExactlySixDecimalsAndMatchesExactIntegerUnits() {
        for (bad in listOf("3", "3.0", "03.000000", "-3.000000", "+3.000000", "3e0", "3.0000000", " 3.000000")) {
            reject(fixture.page().apply { addProperty("total", bad) })
        }
        reject(fixture.page().apply { addProperty("total", 3.000000) })
        reject(fixture.page().apply { addProperty("total_base_units", "3000001") })
    }

    @Test fun maxI64MicroUnitsAcceptedAndOverflowOrUnaccountedUnitRejected() {
        val json = fixture.page(1, 1, 1).apply {
            addProperty("total", "9223372036854.775807")
            addProperty("total_base_units", Long.MAX_VALUE.toString())
            getAsJsonArray("entries")[0].asJsonObject.apply {
                addProperty("amount", "9223372036854.775807")
                addProperty("amount_base_units", Long.MAX_VALUE.toString())
            }
        }
        assertEquals(Long.MAX_VALUE.toString(), parse(json).totalBaseUnits)
        reject(json.deepCopy().apply { addProperty("total", "9223372036854.775808"); addProperty("total_base_units", "9223372036854775808") })
        reject(json.deepCopy().apply { getAsJsonArray("entries")[0].asJsonObject.addProperty("amount_base_units", "1") })
    }

    @Test fun completePageRequiresExactSumAndPartialPageReservesOneUnitPerHiddenEntry() {
        reject(fixture.page(1, 3, 3).apply { addProperty("total", "4.000000"); addProperty("total_base_units", "4000000") })
        val partial = fixture.page().apply { addProperty("total", "2.000001"); addProperty("total_base_units", "2000001") }
        assertEquals("2.000001", parse(partial).total)
        reject(partial.apply { addProperty("total", "2.000000"); addProperty("total_base_units", "2000000") })
        reject(fixture.page().apply { addProperty("entry_count", "3000001") })
    }

    @Test fun rangesAreClosedOneBasedConsistentAndFirstRequestCannotSkip() {
        for ((start, end) in listOf("0" to "2", "2" to "1", "1" to "4", "1" to "1", "2" to "3")) {
            reject(fixture.page().apply { addProperty("range_start", start); addProperty("range_end", end) })
        }
        reject(fixture.page(), fixture.cursor(4))
        reject(fixture.page(3, 3, 3))
        reject(fixture.page(3, 3, 3).apply { addProperty("has_more", true) }, fixture.cursor(2))
        reject(fixture.page().apply { add("entries", JsonArray()) })
    }

    @Test fun digestsAndBothCursorDirectionsAreBoundToThisPage() {
        for (digest in listOf("a".repeat(63), "A".repeat(64), "g".repeat(64))) {
            reject(fixture.page().apply { addProperty("snapshot_digest", digest) })
        }
        for (cursor in listOf("", "ephp0.x", fixture.cursor(2).uppercase(), fixture.cursor(2) + " ")) {
            assertFalse(EskPlatformHistoryParser.validCursor(cursor))
            reject(fixture.page(), cursor)
            reject(fixture.page().apply { addProperty("next_cursor", cursor) })
        }
        reject(fixture.page().apply { addProperty("next_cursor", fixture.cursor(1)) })
        reject(fixture.page().apply { addProperty("next_cursor", fixture.cursor(2, "b".repeat(64))) })
        reject(fixture.page().apply { add("next_cursor", JsonNull.INSTANCE) })
        reject(fixture.page(3, 3, 3).apply { addProperty("next_cursor", fixture.cursor(1)) }, fixture.cursor(2))
        reject(fixture.page(3, 3, 3), fixture.cursor(2, "b".repeat(64)))
        reject(fixture.page(3, 3, 3), fixture.cursor(1))
    }

    @Test fun entriesRequireExactFieldsPositiveAmountAndNoDuplicateIdentities() {
        val entry = fixture.entry(3)
        for (key in entry.keySet()) reject(fixture.page().apply { getAsJsonArray("entries")[0].asJsonObject.remove(key) })
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("extra", "x") }, { it.addProperty("kind", "paper") },
            { it.addProperty("entry_id", fixture.id(3).uppercase()) }, { it.addProperty("allocation_id", "wrong") },
            { it.addProperty("amount", "0.000000"); it.addProperty("amount_base_units", "0") },
            { it.addProperty("amount", 1) }, { it.addProperty("amount_base_units", 1000000) },
            { it.addProperty("entry_id", fixture.id(2)) },
            { it.addProperty("allocation_id", fixture.entry(2)["allocation_id"].asString) },
        )) reject(fixture.page().apply { change(getAsJsonArray("entries")[0].asJsonObject) })
    }

    @Test fun orderUsesRawUtcTimeThenIdDescendingAndNewestSummaryIsGlobal() {
        reject(fixture.page().apply { getAsJsonArray("entries")[0].asJsonObject.addProperty("entry_id", fixture.id(1)) })
        reject(fixture.page().apply { getAsJsonArray("entries")[1].asJsonObject.addProperty("created_at", "2026-09-05T10:00:00+00:00") })
        reject(fixture.page().apply { addProperty("updated_at", "2026-09-05T10:00:00+00:00") })
        reject(fixture.page(3, 3, 3).apply { addProperty("updated_at", "2026-09-03T10:00:00+00:00") }, fixture.cursor(2))
        for (time in listOf("2026-02-30T10:00:00Z", "2026-09-04T10:00:00+08:00", "2026-09-04", "2026-09-04T24:00:00Z")) {
            reject(fixture.page().apply { addProperty("updated_at", time) })
        }
    }

    @Test fun accountAndHistorySchemasCannotBeConfusedInEitherDirection() {
        val error = assertThrows(IllegalArgumentException::class.java) {
            EskPlatformHistoryParser.parse(EskPlatformAccountFixture.response().toByteArray())
        }
        assertEquals("ESK_PLATFORM_HISTORY_INVALID", error.message)
        assertThrows(IllegalArgumentException::class.java) { EskPlatformAccountParser.parse(fixture.page().toString().toByteArray()) }
    }
}
