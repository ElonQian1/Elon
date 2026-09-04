package com.elon.eskcontract

import java.math.BigInteger
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Pure map/number tests; Android Bundle typing, signer checks and replay state are separate. */
class EskSnapshotContractTest {
    private val nonce = "a".repeat(64)
    private val requestedAt = 1_000L
    private val observedAt = 2_000L
    private val now = 3_000L
    private val amountKeys = listOf(
        "total", "available", "reserved_for_sellback", "reserved_for_quant", "reserved_total",
    )

    // SYNTHETIC_WIRE_FIXTURE_START: statically compared with the documented wire vector.
    private fun fixture() = mapOf(
        "protocol" to "yilong.esk.android_snapshot.v1",
        "nonce" to "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "asset_id" to "esk",
        "symbol" to "ESK",
        "mode" to "paper",
        "issuance_mode" to "paper_recorded",
        "chain_status" to "not_deployed",
        "simulated" to "true",
        "funds_moved" to "false",
        "total" to "1250.000000",
        "available" to "900.000000",
        "reserved_for_sellback" to "100.000000",
        "reserved_for_quant" to "250.000000",
        "reserved_total" to "350.000000",
        "revision" to "7",
        "observed_elapsed_ms" to "2000",
        "expires_elapsed_ms" to "62000",
    )
    // SYNTHETIC_WIRE_FIXTURE_END

    private fun accepts(fields: Map<String, String> = fixture()) =
        EskSnapshotContract.validSnapshot(fields, nonce, requestedAt, now)

    @Test
    fun wireHasExactlySeventeenStringFields() {
        assertEquals(17, EskSnapshotContract.KEYS.size)
        assertEquals(EskSnapshotContract.KEYS, fixture().keys)
        assertEquals("com.elon.app.action.READ_ESK_SNAPSHOT", EskSnapshotContract.ACTION)
        assertEquals(120_000L, EskSnapshotContract.REQUEST_WINDOW_MS)
        assertEquals(60_000L, EskSnapshotContract.DISPLAY_WINDOW_MS)
        assertTrue(accepts())
    }

    @Test
    fun requestRequiresExactKeysAndProtocol() {
        val request = mapOf("protocol" to EskSnapshotContract.PROTOCOL, "nonce" to nonce)
        assertTrue(EskSnapshotContract.validRequest(request))
        for (key in request.keys) assertFalse(EskSnapshotContract.validRequest(request - key))
        assertFalse(EskSnapshotContract.validRequest(request + ("extra" to "value")))
        assertFalse(EskSnapshotContract.validRequest(request + ("protocol" to "v2")))
        assertFalse(EskSnapshotContract.validRequest(emptyMap()))
    }

    @Test
    fun nonceMustBeExactlyLowercaseHexAndMatchCurrentRequest() {
        val invalid = listOf("", "a".repeat(63), "a".repeat(65), "A".repeat(64),
            "g".repeat(64), "0x" + nonce, nonce + "\n")
        for (value in invalid) {
            assertFalse(value, EskSnapshotContract.validRequest(
                mapOf("protocol" to EskSnapshotContract.PROTOCOL, "nonce" to value),
            ))
            assertFalse(value, EskSnapshotContract.validSnapshot(fixture(), value, requestedAt, now))
            assertFalse(value, accepts(fixture() + ("nonce" to value)))
        }
        assertFalse(accepts(fixture() + ("nonce" to "b".repeat(64))))
    }

    @Test
    fun snapshotRejectsEveryMissingFieldAndUnknownFields() {
        for (key in fixture().keys) assertFalse(key, accepts(fixture() - key))
        assertFalse(accepts(fixture() + ("user_id" to "synthetic")))
        assertFalse(accepts(fixture() + ("access_token" to "synthetic")))
        assertFalse(accepts(emptyMap()))
    }

    @Test
    fun snapshotRejectsOversizedValuesWithoutTruncation() {
        for (key in fixture().keys) {
            assertFalse(key, accepts(fixture() + (key to "x".repeat(129))))
        }
    }

    @Test
    fun amountsAcceptSixDecimalsAndExactLongMaximumMicroUnits() {
        assertEquals(BigInteger.ZERO, EskSnapshotContract.units("0.000000"))
        assertEquals(BigInteger.ONE, EskSnapshotContract.units("0.000001"))
        assertEquals(BigInteger.valueOf(1_234_567L), EskSnapshotContract.units("1.234567"))
        assertEquals(BigInteger.valueOf(Long.MAX_VALUE),
            EskSnapshotContract.units("9223372036854.775807"))
    }

    @Test
    fun amountsRejectNegativeExponentRoundingAndNonCanonicalText() {
        val invalid = listOf("", "0", "1.0", ".000001", "1.0000000", "00.000000",
            "01.000000", "-1.000000", "-0.000000", "+1.000000", "1e6", "NaN",
            "Infinity", "1,000.000000", " 1.000000", "1.000000 ", "1.000000\n",
            "１.000000", "9223372036854.775808", "10000000000000.000000")
        for (value in invalid) {
            assertNull(value, EskSnapshotContract.units(value))
            for (key in amountKeys) assertFalse("$key=$value", accepts(fixture() + (key to value)))
        }
    }

    @Test
    fun balancesRequireBothConservationEquations() {
        assertTrue(EskSnapshotContract.validBalances(fixture()))
        assertFalse(accepts(fixture() + ("total" to "1250.000001")))
        assertFalse(accepts(fixture() + ("reserved_total" to "349.000000")))
        assertFalse(accepts(fixture() + ("reserved_for_quant" to "251.000000")))
        assertFalse(accepts(fixture() + ("available" to "1250.000000")))
        for (key in amountKeys) assertFalse(EskSnapshotContract.validBalances(fixture() - key))
    }

    @Test
    fun zeroAndLongMaximumBalancesRemainExact() {
        val zero = fixture() + amountKeys.associateWith { "0.000000" }
        assertTrue(accepts(zero))
        val maximum = zero + mapOf(
            "total" to "9223372036854.775807", "available" to "9223372036854.775807",
        )
        assertTrue(accepts(maximum))
        // The sum is evaluated as BigInteger; it must not wrap around Long.MAX_VALUE.
        assertFalse(accepts(maximum + mapOf(
            "reserved_total" to "0.000001", "reserved_for_quant" to "0.000001",
        )))
    }

    @Test
    fun integerAndRevisionAreCanonicalNonNegativeLongStrings() {
        assertEquals(0L, EskSnapshotContract.integer("0"))
        assertEquals(Long.MAX_VALUE, EskSnapshotContract.integer(Long.MAX_VALUE.toString()))
        assertTrue(accepts(fixture() + ("revision" to "0")))
        assertTrue(accepts(fixture() + ("revision" to Long.MAX_VALUE.toString())))
        val invalid = listOf("", "-1", "+1", "00", "01", "1.0", "1e3", " 1", "1 ",
            "1\n", "9223372036854775808", "10000000000000000000")
        for (value in invalid) {
            assertNull(value, EskSnapshotContract.integer(value))
            assertFalse(value, accepts(fixture() + ("revision" to value)))
        }
    }

    @Test
    fun onlyPaperAndDisabledModesAreAccepted() {
        assertTrue(accepts(fixture() + ("mode" to "disabled")))
        for (value in listOf("live", "sandbox", "testnet", "Paper", "", "paper ")) {
            assertFalse(value, accepts(fixture() + ("mode" to value)))
        }
    }

    @Test
    fun snapshotCannotClaimDeploymentFundsOrAnotherAsset() {
        val forbidden = mapOf(
            "protocol" to "yilong.esk.android_snapshot.v2", "asset_id" to "qshare",
            "symbol" to "USDT", "issuance_mode" to "onchain",
            "chain_status" to "deployed", "simulated" to "false", "funds_moved" to "true",
        )
        for ((key, value) in forbidden) assertFalse(key, accepts(fixture() + (key to value)))
        assertFalse(accepts(fixture() + ("simulated" to "True")))
        assertFalse(accepts(fixture() + ("funds_moved" to "0")))
    }

    @Test
    fun requestWindowIsStrictlyLessThanOneHundredTwentySeconds() {
        assertTrue(EskSnapshotContract.validWindow(0L, 0L))
        assertTrue(EskSnapshotContract.validWindow(0L, 119_999L))
        assertFalse(EskSnapshotContract.validWindow(0L, 120_000L))
        assertFalse(EskSnapshotContract.validWindow(0L, Long.MAX_VALUE))
        val late = fixture() + mapOf(
            "observed_elapsed_ms" to "119999", "expires_elapsed_ms" to "179999",
        )
        assertTrue(EskSnapshotContract.validSnapshot(late, nonce, 0L, 119_999L))
        assertFalse(EskSnapshotContract.validSnapshot(late, nonce, 0L, 120_000L))
    }

    @Test
    fun negativeAndReversedMonotonicTimesFailClosed() {
        assertFalse(EskSnapshotContract.validWindow(-1L, 0L))
        assertFalse(EskSnapshotContract.validWindow(2L, 1L))
        assertFalse(EskSnapshotContract.validWindow(Long.MAX_VALUE, Long.MIN_VALUE))
        assertFalse(EskSnapshotContract.validSnapshot(fixture(), nonce, -1L, now))
        assertFalse(EskSnapshotContract.validSnapshot(fixture(), nonce, requestedAt, requestedAt - 1L))
    }

    @Test
    fun observationMustBeBetweenRequestAndCurrentTimeInclusive() {
        assertTrue(EskSnapshotContract.validSnapshot(fixture(), nonce, observedAt, observedAt))
        assertFalse(accepts(fixture() + mapOf(
            "observed_elapsed_ms" to "999", "expires_elapsed_ms" to "60999",
        )))
        assertFalse(accepts(fixture() + mapOf(
            "observed_elapsed_ms" to "3001", "expires_elapsed_ms" to "63001",
        )))
    }

    @Test
    fun displayTtlIsExactlySixtySecondsAndExpiryIsExclusive() {
        assertTrue(EskSnapshotContract.validSnapshot(fixture(), nonce, requestedAt, 61_999L))
        assertFalse(EskSnapshotContract.validSnapshot(fixture(), nonce, requestedAt, 62_000L))
        assertFalse(EskSnapshotContract.validSnapshot(fixture(), nonce, requestedAt, 62_001L))
        for (expires in listOf("61999", "62001", "2000", "1999", "0")) {
            assertFalse(expires, accepts(fixture() + ("expires_elapsed_ms" to expires)))
        }
    }

    @Test
    fun timestampParsingRejectsOverflowSignsAndFractionalValues() {
        for (key in listOf("observed_elapsed_ms", "expires_elapsed_ms")) {
            for (value in listOf("-1", "1.0", "1e3", "01", "9223372036854775808")) {
                assertFalse("$key=$value", accepts(fixture() + (key to value)))
            }
        }
    }

    @Test
    fun validTimesNearLongMaximumDoNotOverflow() {
        val observed = Long.MAX_VALUE - EskSnapshotContract.DISPLAY_WINDOW_MS
        val fields = fixture() + mapOf(
            "observed_elapsed_ms" to observed.toString(),
            "expires_elapsed_ms" to Long.MAX_VALUE.toString(),
        )
        assertTrue(EskSnapshotContract.validSnapshot(fields, nonce, observed, Long.MAX_VALUE - 1L))
        assertFalse(EskSnapshotContract.validSnapshot(fields, nonce, observed, Long.MAX_VALUE))
        assertTrue(EskSnapshotContract.validWindow(Long.MAX_VALUE - 119_999L, Long.MAX_VALUE))
        assertFalse(EskSnapshotContract.validWindow(Long.MAX_VALUE - 120_000L, Long.MAX_VALUE))
    }
}
