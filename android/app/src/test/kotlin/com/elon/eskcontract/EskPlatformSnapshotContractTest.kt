package com.elon.eskcontract

import java.math.BigInteger
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Synthetic Map tests only: actual Bundle types, caller identity and replay state are external. */
class EskPlatformSnapshotContractTest {
    private val nonce = "a".repeat(64)
    private val requestedAt = 1_000L
    private val now = 3_000L

    // SYNTHETIC_WIRE_FIXTURE_START
    private fun fixture() = mapOf(
        "protocol" to "yilong.esk.platform_android_snapshot.v1",
        "nonce" to "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "asset_id" to "esk",
        "symbol" to "ESK",
        "decimals" to "6",
        "source" to "platform_recorded",
        "chain_status" to "not_deployed",
        "simulated" to "false",
        "funds_moved" to "false",
        "verification_basis" to "authenticated_operator_review",
        "external_payment_verified" to "false",
        "total" to "1250.000000",
        "total_base_units" to "1250000000",
        "entry_count" to "7",
        "observed_elapsed_ms" to "2000",
        "expires_elapsed_ms" to "62000",
        "service_spending" to "false",
        "quant_subscription" to "false",
        "sellback_settlement" to "false",
        "onchain_transfer" to "false",
        "chain_migration" to "false",
    )
    // SYNTHETIC_WIRE_FIXTURE_END

    private fun accepts(fields: Map<String, String> = fixture()) =
        EskPlatformSnapshotContract.validSnapshot(fields, nonce, requestedAt, now)

    @Test fun fixedProtocolContainsExactlyTwentyOneStringFields() {
        assertEquals(21, EskPlatformSnapshotContract.KEYS.size)
        assertEquals(EskPlatformSnapshotContract.KEYS, fixture().keys)
        assertEquals("com.elon.app.action.READ_ESK_PLATFORM_SNAPSHOT", EskPlatformSnapshotContract.ACTION)
        assertEquals(120_000L, EskPlatformSnapshotContract.REQUEST_WINDOW_MS)
        assertEquals(60_000L, EskPlatformSnapshotContract.DISPLAY_WINDOW_MS)
        assertTrue(accepts())
    }

    @Test fun requestHasExactKeysAndIndependentVersion() {
        val request = mapOf("protocol" to EskPlatformSnapshotContract.PROTOCOL, "nonce" to nonce)
        assertTrue(EskPlatformSnapshotContract.validRequest(request))
        for (key in request.keys) assertFalse(EskPlatformSnapshotContract.validRequest(request - key))
        assertFalse(EskPlatformSnapshotContract.validRequest(request + ("source" to "platform_recorded")))
        assertFalse(EskPlatformSnapshotContract.validRequest(request + ("protocol" to "v2")))
        assertFalse(EskPlatformSnapshotContract.validRequest(emptyMap()))
        assertFalse(EskSnapshotContract.validRequest(request))
        assertFalse(EskPlatformSnapshotContract.validRequest(request + ("protocol" to EskSnapshotContract.PROTOCOL)))
    }

    @Test fun nonceRequiresLowercaseHexAndCurrentRequestIdentity() {
        for (value in listOf("", "a".repeat(63), "a".repeat(65), "A".repeat(64), "g".repeat(64),
            "0x$nonce", "$nonce\n", " $nonce")) {
            assertFalse(EskPlatformSnapshotContract.validRequest(mapOf(
                "protocol" to EskPlatformSnapshotContract.PROTOCOL, "nonce" to value)))
            assertFalse(EskPlatformSnapshotContract.validSnapshot(fixture(), value, requestedAt, now))
            assertFalse(accepts(fixture() + ("nonce" to value)))
        }
        assertFalse(accepts(fixture() + ("nonce" to "b".repeat(64))))
    }

    @Test fun missingUnknownAndOversizedFieldsAreRejected() {
        for (key in fixture().keys) {
            assertFalse(key, accepts(fixture() - key))
            assertFalse(key, accepts(fixture() + (key to "x".repeat(129))))
        }
        for (key in listOf("user_id", "nickname", "token", "available", "revision", "entries")) {
            assertFalse(key, accepts(fixture() + (key to "synthetic")))
        }
        assertFalse(accepts(emptyMap()))
    }

    @Test fun sourcesAssetAndVerificationCannotBeSubstituted() {
        for ((key, value) in listOf("protocol" to "yilong.esk.platform_android_snapshot.v2",
            "asset_id" to "qshare", "symbol" to "USDT", "decimals" to "6.0", "decimals" to "06",
            "source" to "paper_recorded", "source" to "migration_pending", "source" to "onchain_verified",
            "chain_status" to "deployed", "verification_basis" to "chain_proof")) {
            assertFalse(key, accepts(fixture() + (key to value)))
        }
    }

    @Test fun allFlagsAndCapabilitiesAreExactlyFalseStrings() {
        val fields = listOf("simulated", "funds_moved", "external_payment_verified", "service_spending",
            "quant_subscription", "sellback_settlement", "onchain_transfer", "chain_migration")
        for (key in fields) {
            for (value in listOf("true", "False", "0", "", " false")) {
                assertFalse(key, accepts(fixture() + (key to value)))
            }
        }
    }

    @Test fun sixDecimalAmountsRetainExactMicroUnitsAndI64Maximum() {
        assertEquals(BigInteger.ZERO, EskPlatformSnapshotContract.units("0.000000"))
        assertEquals(BigInteger.ONE, EskPlatformSnapshotContract.units("0.000001"))
        assertEquals(BigInteger.valueOf(1_234_567L), EskPlatformSnapshotContract.units("1.234567"))
        assertEquals(BigInteger.valueOf(Long.MAX_VALUE), EskPlatformSnapshotContract.units("9223372036854.775807"))
        val maximum = fixture() + mapOf("total" to "9223372036854.775807",
            "total_base_units" to Long.MAX_VALUE.toString(), "entry_count" to Long.MAX_VALUE.toString())
        assertTrue(accepts(maximum))
    }

    @Test fun amountsRejectSignsExponentsRoundingAndNoncanonicalText() {
        for (value in listOf("", "0", "1.0", ".000001", "1.0000000", "00.000000", "01.000000",
            "-1.000000", "-0.000000", "+1.000000", "1e6", "NaN", "Infinity", "1,000.000000",
            " 1.000000", "1.000000 ", "1.000000\n", "１.000000", "9223372036854.775808",
            "10000000000000.000000")) {
            assertNull(value, EskPlatformSnapshotContract.units(value))
            assertFalse(value, accepts(fixture() + ("total" to value)))
        }
    }

    @Test fun canonicalIntegerRejectsOverflowAndCoercion() {
        assertEquals(0L, EskPlatformSnapshotContract.integer("0"))
        assertEquals(Long.MAX_VALUE, EskPlatformSnapshotContract.integer(Long.MAX_VALUE.toString()))
        val keys = listOf("total_base_units", "entry_count", "observed_elapsed_ms", "expires_elapsed_ms")
        for (value in listOf("", "-1", "+1", "-0", "00", "01", "1.0", "1e3", " 1", "1 ",
            "1\n", "9223372036854775808", "10000000000000000000")) {
            assertNull(value, EskPlatformSnapshotContract.integer(value))
            for (key in keys) assertFalse(key, accepts(fixture() + (key to value)))
        }
    }

    @Test fun zeroCountIfAndOnlyIfTotalIsZero() {
        val zero = fixture() + mapOf("total" to "0.000000", "total_base_units" to "0", "entry_count" to "0")
        assertTrue(accepts(zero))
        assertFalse(accepts(zero + ("entry_count" to "1")))
        assertFalse(accepts(fixture() + ("entry_count" to "0")))
    }

    @Test fun amountUnitsAndPositiveEntryCountMustAgree() {
        assertFalse(accepts(fixture() + ("total_base_units" to "1250000001")))
        assertFalse(accepts(fixture() + ("total" to "1250.000001")))
        val one = fixture() + mapOf("total" to "0.000001", "total_base_units" to "1", "entry_count" to "1")
        assertTrue(accepts(one))
        assertFalse(accepts(one + ("entry_count" to "2")))
        assertFalse(accepts(fixture() + ("entry_count" to Long.MAX_VALUE.toString())))
    }

    @Test fun requestWindowIsStrictAndSafeAtLongMaximum() {
        assertTrue(EskPlatformSnapshotContract.validWindow(0L, 0L))
        assertTrue(EskPlatformSnapshotContract.validWindow(0L, 119_999L))
        assertFalse(EskPlatformSnapshotContract.validWindow(0L, 120_000L))
        assertFalse(EskPlatformSnapshotContract.validWindow(0L, Long.MAX_VALUE))
        assertTrue(EskPlatformSnapshotContract.validWindow(Long.MAX_VALUE - 119_999L, Long.MAX_VALUE))
        assertFalse(EskPlatformSnapshotContract.validWindow(Long.MAX_VALUE - 120_000L, Long.MAX_VALUE))
        val late = fixture() + mapOf("observed_elapsed_ms" to "119999", "expires_elapsed_ms" to "179999")
        assertTrue(EskPlatformSnapshotContract.validSnapshot(late, nonce, 0L, 119_999L))
        assertFalse(EskPlatformSnapshotContract.validSnapshot(late, nonce, 0L, 120_000L))
    }

    @Test fun negativeReversedAndOutOfOrderTimesAreRejected() {
        assertFalse(EskPlatformSnapshotContract.validWindow(-1L, 0L))
        assertFalse(EskPlatformSnapshotContract.validWindow(2L, 1L))
        assertFalse(EskPlatformSnapshotContract.validWindow(Long.MAX_VALUE, Long.MIN_VALUE))
        assertFalse(EskPlatformSnapshotContract.validSnapshot(fixture(), nonce, -1L, now))
        assertFalse(EskPlatformSnapshotContract.validSnapshot(fixture(), nonce, requestedAt, requestedAt - 1L))
        assertFalse(accepts(fixture() + ("observed_elapsed_ms" to "999")))
        assertFalse(accepts(fixture() + ("observed_elapsed_ms" to "3001")))
        assertTrue(EskPlatformSnapshotContract.validSnapshot(fixture(), nonce, 2_000L, 2_000L))
    }

    @Test fun displayTtlCanBeShortenedToOneMillisecond() {
        for (ttl in listOf(1L, 2L, 59_999L, 60_000L)) {
            val fields = fixture() + ("expires_elapsed_ms" to (2_000L + ttl).toString())
            assertTrue(EskPlatformSnapshotContract.validSnapshot(fields, nonce, requestedAt, 2_000L))
            assertTrue(EskPlatformSnapshotContract.validSnapshot(fields, nonce, requestedAt, 1_999L + ttl))
            assertFalse(EskPlatformSnapshotContract.validSnapshot(fields, nonce, requestedAt, 2_000L + ttl))
        }
    }

    @Test fun nonpositiveOrExcessiveTtlAndExactExpiryAreRejected() {
        for (expires in listOf("0", "1999", "2000", "62001", Long.MAX_VALUE.toString())) {
            assertFalse(expires, EskPlatformSnapshotContract.validSnapshot(
                fixture() + ("expires_elapsed_ms" to expires), nonce, requestedAt, 2_000L))
        }
        assertTrue(EskPlatformSnapshotContract.validSnapshot(fixture(), nonce, requestedAt, 61_999L))
        assertFalse(EskPlatformSnapshotContract.validSnapshot(fixture(), nonce, requestedAt, 62_000L))
    }

    @Test fun validTimeDifferencesNearLongMaximumDoNotOverflow() {
        for (ttl in listOf(1L, 60_000L)) {
            val observed = Long.MAX_VALUE - ttl
            val fields = fixture() + mapOf("observed_elapsed_ms" to observed.toString(),
                "expires_elapsed_ms" to Long.MAX_VALUE.toString())
            assertTrue(EskPlatformSnapshotContract.validSnapshot(fields, nonce, observed, Long.MAX_VALUE - 1L))
            assertFalse(EskPlatformSnapshotContract.validSnapshot(fields, nonce, observed, Long.MAX_VALUE))
        }
    }

    @Test fun pureValidatorDoesNotPretendToConsumeNonceOrKnowAccountIdentity() {
        assertTrue(accepts())
        assertTrue(accepts()) // Android owner must consume once and authenticate caller/session separately.
        assertFalse(EskPlatformSnapshotContract.KEYS.any { it in setOf("user_id", "account", "nickname", "token") })
    }

    @Test fun oldPaperAndFormalSnapshotsRejectEachOtherWithoutFallback() {
        val paper = mapOf("protocol" to EskSnapshotContract.PROTOCOL, "nonce" to nonce,
            "asset_id" to "esk", "symbol" to "ESK", "mode" to "paper", "issuance_mode" to "paper_recorded",
            "chain_status" to "not_deployed", "simulated" to "true", "funds_moved" to "false",
            "total" to "1.000000", "available" to "1.000000", "reserved_for_sellback" to "0.000000",
            "reserved_for_quant" to "0.000000", "reserved_total" to "0.000000", "revision" to "1",
            "observed_elapsed_ms" to "2000", "expires_elapsed_ms" to "62000")
        assertTrue(EskSnapshotContract.validSnapshot(paper, nonce, requestedAt, now))
        assertFalse(accepts(paper))
        assertFalse(accepts(paper + ("protocol" to EskPlatformSnapshotContract.PROTOCOL)))
        assertFalse(EskSnapshotContract.validSnapshot(fixture(), nonce, requestedAt, now))
        assertFalse(EskSnapshotContract.validSnapshot(fixture() + ("protocol" to EskSnapshotContract.PROTOCOL),
            nonce, requestedAt, now))
        assertFalse(EskPlatformSnapshotContract.ACTION == EskSnapshotContract.ACTION)
    }
}
