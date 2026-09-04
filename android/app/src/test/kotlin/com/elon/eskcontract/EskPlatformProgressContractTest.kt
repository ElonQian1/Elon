package com.elon.eskcontract

import com.elon.eskcontract.EskPlatformProgressFixtures.NONCE
import com.elon.eskcontract.EskPlatformProgressFixtures.accepts
import com.elon.eskcontract.EskPlatformProgressFixtures.cursor
import com.elon.eskcontract.EskPlatformProgressFixtures.page
import com.elon.eskcontract.EskPlatformProgressFixtures.row
import java.math.BigInteger
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Synthetic pure Map validation, not an OS Bundle, caller, session or real-account acceptance test. */
class EskPlatformProgressContractTest {
    private val contract = EskPlatformProgressContract
    private fun request(cursor: String = "") = mapOf("protocol" to contract.PROTOCOL, "nonce" to NONCE, "cursor" to cursor)

    @Test fun exactIndependentProtocolAndBoundedKeyGeneration() {
        assertEquals("yilong.esk.platform_android_progress.v1", contract.PROTOCOL)
        assertEquals("com.elon.app.action.READ_ESK_PLATFORM_PROGRESS", contract.ACTION)
        assertEquals(setOf("protocol", "nonce", "cursor"), contract.REQUEST_KEYS)
        assertEquals(35, contract.TOP_KEYS.size)
        assertEquals(35, contract.keysForCount(0).size)
        assertEquals(155, contract.keysForCount(20).size)
        assertEquals(155, contract.MAX_KEYS)
        assertEquals(128, contract.MAX_VALUE_LENGTH)
        assertEquals(64, contract.MAX_KEY_LENGTH)
        assertEquals(32768, contract.MAX_BYTES)
        assertEquals(20, contract.MAX_PAGE_COUNT)
        for (n in listOf(-1, 21, Int.MIN_VALUE, Int.MAX_VALUE)) assertTrue(contract.keysForCount(n).isEmpty())
        assertEquals(contract.keysForCount(2), page().keys)
        assertTrue(accepts())
    }

    @Test fun requestRequiresThreeKeysAndFreshNonceShape() {
        assertTrue(contract.validRequest(request()))
        assertTrue(contract.validRequest(request(cursor(3))))
        for (key in request().keys) assertFalse(contract.validRequest(request() - key))
        assertFalse(contract.validRequest(request() + ("token" to "synthetic")))
        for (nonce in listOf("", "a".repeat(63), "a".repeat(65), "A".repeat(64), "g".repeat(64), "$NONCE\n")) {
            assertFalse(contract.validRequest(request() + ("nonce" to nonce)))
            assertFalse(contract.validSnapshot(page(), nonce, "", 1000, 3000))
            assertFalse(accepts(page() + ("nonce" to nonce)))
        }
        assertFalse(accepts(page() + ("nonce" to "c".repeat(64))))
    }

    @Test fun cursorHasExactAsciiShapeAndCannotBeChangedByResponse() {
        assertEquals(110, cursor(3).length)
        for (bad in listOf(" ", cursor(3) + "\n", cursor(3).uppercase(), cursor(3).dropLast(1),
            cursor(3).replace("esbr1", "esbr2"), cursor(3).replace("eskpsr_", "eskpsc_"), "x".repeat(129))) {
            assertFalse(bad, contract.validCursor(bad))
            assertFalse(contract.validRequest(request(bad)))
            assertFalse(contract.validSnapshot(page(), NONCE, bad, 1000, 3000))
        }
        assertFalse(accepts(page() + ("requested_cursor" to cursor(3))))
        assertFalse(contract.validSnapshot(page(), NONCE, cursor(3), 1000, 3000))
    }

    @Test fun exactShapeRejectsEveryMissingFieldAndSensitiveExtension() {
        for (key in page().keys) assertFalse(key, accepts(page() - key))
        for (key in listOf("user_id", "token", "nickname", "policy", "idempotency_key", "entries", "request_2_id",
            "request_00_id", "request_0_key", "request_-1_status", "request_0_policy_digest")) {
            assertFalse(key, accepts(page() + (key to "synthetic")))
        }
        assertFalse(accepts(emptyMap()))
        assertFalse(accepts(page() + ("page_count" to "1")))
        assertFalse(accepts(page() + ("page_count" to "3")))
        assertFalse(accepts(page(List(21) { row(30 - it) }, reserved = 21, total = 21, open = 21)))
    }

    @Test fun countAndTextBudgetsAreAppliedBeforeDynamicFieldParsing() {
        for (key in page().keys) assertFalse(key, accepts(page() + (key to "x".repeat(129))))
        assertFalse(accepts(page() + ("k".repeat(65) to "x")))
        val huge = (0..155).associate { "$it" to "" }
        assertFalse(accepts(huge))
        // Each value is within the character cap, but raw UTF-8 key/value bytes exceed the total cap.
        val wide = (0..154).associate { "k$it" to "界".repeat(128) }
        assertTrue(wide.entries.sumOf { it.key.toByteArray().size + it.value.toByteArray().size } > contract.MAX_BYTES)
        assertFalse(accepts(wide))
        @Suppress("UNCHECKED_CAST")
        val wrongType = (page().toMutableMap() as MutableMap<Any?, Any?>).apply { put("page_count", 2) }
        @Suppress("UNCHECKED_CAST")
        assertFalse(accepts(wrongType as Map<String, String>))
        @Suppress("UNCHECKED_CAST")
        val nullRequest = mapOf("protocol" to contract.PROTOCOL, "nonce" to NONCE, "cursor" to null) as Map<String, String>
        assertFalse(contract.validRequest(nullRequest))
    }

    @Test fun immutableSourceAndAllWriteFlagsRejectSubstitution() {
        for ((key, bad) in listOf("protocol" to "v2", "asset_id" to "usdt", "symbol" to "USDT", "decimals" to "06",
            "source" to "paper_recorded", "chain_status" to "deployed", "verification_basis" to "chain_verified",
            "snapshot_digest" to "A".repeat(64), "snapshot_digest" to "b".repeat(63))) {
            assertFalse(key, accepts(page() + (key to bad)))
        }
        for (key in listOf("simulated", "funds_moved", "external_payment_verified", "service_spending", "quant_subscription",
            "sellback_settlement", "onchain_transfer", "chain_migration", "submit_request", "cancel_request")) {
            for (bad in listOf("true", "False", "0", "", " false")) assertFalse(key, accepts(page() + (key to bad)))
        }
    }

    @Test fun sixDecimalMicroUnitsAreCanonicalAndAllowLongMaximum() {
        assertEquals(BigInteger.ZERO, contract.units("0.000000"))
        assertEquals(BigInteger.ONE, contract.units("0.000001"))
        assertEquals(BigInteger.valueOf(Long.MAX_VALUE), contract.units("9223372036854.775807"))
        assertEquals(Long.MAX_VALUE, contract.integer("9223372036854775807"))
        assertTrue(accepts(page(listOf(row(1, Long.MAX_VALUE)), total = Long.MAX_VALUE, reserved = Long.MAX_VALUE)))
        val badAmounts = listOf("0", "1.0", "00.000000", ".000001", "+1.000000", "-0.000000", "1e6", "NaN",
            "1.0000000", "1.000000\n", "１.000000", "9223372036854.775808", "10000000000000.000000")
        for (value in badAmounts) {
            assertNull(value, contract.units(value))
            for (key in listOf("total", "reserved", "available", "request_0_amount")) assertFalse(key, accepts(page() + (key to value)))
        }
    }

    @Test fun integersRejectNegativeCoercionOverflowAndNoncanonicalForms() {
        val keys = listOf("total_base_units", "reserved_base_units", "available_base_units", "request_count", "open_count",
            "range_start", "range_end", "page_count", "observed_elapsed_ms", "expires_elapsed_ms", "request_0_amount_base_units")
        for (value in listOf("", "-1", "+1", "00", "01", "1.0", "1e3", " 1", "1\n", "１", "9223372036854775808")) {
            assertNull(value, contract.integer(value))
            for (key in keys) assertFalse(key, accepts(page() + (key to value)))
        }
    }

    @Test fun allAmountPairsAndConservationMustAgreeExactly() {
        for (key in listOf("total", "reserved", "available")) {
            assertFalse(key, accepts(page() + (key to "0.000001")))
            assertFalse(key, accepts(page() + ("${key}_base_units" to "1")))
        }
        assertFalse(accepts(page() + mapOf("available" to "0.000008", "available_base_units" to "8")))
        assertFalse(accepts(page() + mapOf("reserved" to "0.000011", "reserved_base_units" to "11")))
        assertFalse(accepts(page() + ("open_count" to "0")))
        assertFalse(accepts(page() + ("open_count" to "3")))
        assertFalse(accepts(page() + ("open_count" to "11")))
    }

    @Test fun windowBoundariesAndOrderingAreStrictWithoutOverflow() {
        assertEquals(120000L, contract.REQUEST_WINDOW_MS)
        assertEquals(60000L, contract.DISPLAY_WINDOW_MS)
        assertTrue(contract.validWindow(0, 119999))
        assertFalse(contract.validWindow(0, 120000))
        assertFalse(contract.validWindow(-1, 0))
        assertFalse(contract.validWindow(2, 1))
        assertFalse(contract.validWindow(Long.MAX_VALUE, Long.MIN_VALUE))
        assertTrue(contract.validWindow(Long.MAX_VALUE - 119999, Long.MAX_VALUE))
        val late = page() + mapOf("observed_elapsed_ms" to "119999", "expires_elapsed_ms" to "179999")
        assertTrue(contract.validSnapshot(late, NONCE, "", 0, 119999))
        assertFalse(contract.validSnapshot(late, NONCE, "", 0, 120000))
        for (observed in listOf("999", "3001")) assertFalse(accepts(page() + ("observed_elapsed_ms" to observed)))
        assertFalse(contract.validSnapshot(page(), NONCE, "", -1, 3000))
        assertFalse(contract.validSnapshot(page(), NONCE, "", 4000, 3000))
    }

    @Test fun shortenedDisplayWindowsAndLongMaximumStayBounded() {
        for (ttl in listOf(1L, 2L, 59999L, 60000L)) {
            val values = page() + ("expires_elapsed_ms" to (2000 + ttl).toString())
            assertTrue(contract.validSnapshot(values, NONCE, "", 1000, 2000))
            assertTrue(contract.validSnapshot(values, NONCE, "", 1000, 1999 + ttl))
            assertFalse(contract.validSnapshot(values, NONCE, "", 1000, 2000 + ttl))
        }
        for (expires in listOf("0", "1999", "2000", "62001", Long.MAX_VALUE.toString())) {
            assertFalse(contract.validSnapshot(page() + ("expires_elapsed_ms" to expires), NONCE, "", 1000, 2000))
        }
        val nearMax = page() + mapOf("observed_elapsed_ms" to (Long.MAX_VALUE - 60000).toString(), "expires_elapsed_ms" to Long.MAX_VALUE.toString())
        assertTrue(contract.validSnapshot(nearMax, NONCE, "", Long.MAX_VALUE - 60000, Long.MAX_VALUE - 1))
        assertFalse(contract.validSnapshot(nearMax, NONCE, "", Long.MAX_VALUE - 60000, Long.MAX_VALUE))
    }

    @Test fun legacyProtocolsMutuallyRejectEvenWithRetagging() {
        val formal = page().filterKeys { it in EskPlatformSnapshotContract.KEYS } + mapOf(
            "protocol" to EskPlatformSnapshotContract.PROTOCOL, "entry_count" to "1")
        val paper = mapOf("protocol" to EskSnapshotContract.PROTOCOL, "nonce" to NONCE, "asset_id" to "esk", "symbol" to "ESK",
            "mode" to "paper", "issuance_mode" to "paper_recorded", "chain_status" to "not_deployed", "simulated" to "true",
            "funds_moved" to "false", "total" to "1.000000", "available" to "1.000000", "reserved_for_sellback" to "0.000000",
            "reserved_for_quant" to "0.000000", "reserved_total" to "0.000000", "revision" to "1",
            "observed_elapsed_ms" to "2000", "expires_elapsed_ms" to "62000")
        assertTrue(EskPlatformSnapshotContract.validSnapshot(formal, NONCE, 1000, 3000))
        assertTrue(EskSnapshotContract.validSnapshot(paper, NONCE, 1000, 3000))
        for (old in listOf(formal, paper)) {
            assertFalse(accepts(old))
            assertFalse(accepts(old + ("protocol" to contract.PROTOCOL)))
            assertFalse(contract.validRequest(old.filterKeys { it in setOf("protocol", "nonce") }))
        }
        assertFalse(EskPlatformSnapshotContract.validSnapshot(page(), NONCE, 1000, 3000))
        assertFalse(EskSnapshotContract.validSnapshot(page(), NONCE, 1000, 3000))
        assertFalse(EskPlatformSnapshotContract.validSnapshot(page() + ("protocol" to EskPlatformSnapshotContract.PROTOCOL), NONCE, 1000, 3000))
        assertFalse(EskSnapshotContract.validSnapshot(page() + ("protocol" to EskSnapshotContract.PROTOCOL), NONCE, 1000, 3000))
        assertFalse(EskPlatformSnapshotContract.validRequest(request()))
        assertFalse(EskSnapshotContract.validRequest(request()))
    }

    @Test fun validationIsPureAndCannotAuthenticateOrConsumeConsent() {
        val snapshot = page()
        assertTrue(accepts(snapshot))
        assertTrue(accepts(snapshot)) // The surrounding native session must consume once.
        assertEquals(page(), snapshot)
        assertFalse(contract.TOP_KEYS.any { it in setOf("user_id", "nickname", "token", "idempotency_key", "policy") })
    }
}
