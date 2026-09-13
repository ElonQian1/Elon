package com.elon.app.exchange.okx

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class OkxReadProjectionTest {
    private val row = mapOf("algoId" to "1234567890123456789", "algoOrdType" to "contract_grid", "instId" to "BTC-USDT-SWAP",
        "state" to "running", "minPx" to "60000.00", "maxPx" to "70000.00", "gridNum" to "20", "cTime" to "1700000000000",
        "direction" to "long", "runType" to "1", "lever" to "3", "pnl" to "-13.375", "totalPnl" to "-13.375")
    @Test fun sharedCanonicalFixtureMatchesExistingReadModel() {
        val actual = OkxReadProjection.encode(OkxAccount("a".repeat(64), "sub"), 7, 1, 1800000000100,
            listOf(OkxReadProjection.bot(row, 7, 1800000000100, false)), null)
        val expected = javaClass.getResourceAsStream("/okx-mobile-read-v1.fixture.json")!!.bufferedReader().use { it.readText() }
        // org.json compares JSON semantics, including exact decimal strings and missing-reason fields.
        assertTrue(org.json.JSONObject(expected).similar(org.json.JSONObject(actual)))
        assertFalse(actual.contains("-13.375"))
    }
    @Test fun malformedOrUnsupportedRowsAreRejectedInsteadOfFaked() {
        for (patch in listOf(mapOf("minPx" to "70000"), mapOf("gridNum" to "0"), mapOf("pnl" to "NaN"),
            mapOf("instId" to "BTC-USD-SWAP"), mapOf("cTime" to "9999999999999"), mapOf("settleCcy" to "BTC"))) {
            assertThrows(IllegalArgumentException::class.java) { OkxReadProjection.bot(row + patch, 7, 1800000000100, false) }
        }
    }
    @Test fun unknownStatusAndDirectionRemainUnknown() {
        val projected = OkxReadProjection.bot(row + mapOf("state" to "starting", "direction" to "future_direction"), 7, 1800000000100, true)
        val encoded = StrictJson.encode(projected)
        assertTrue(encoded.contains("\"status\":\"unknown\"")); assertTrue(encoded.contains("\"missing_reason\":\"not_verified\""))
    }
    @Test fun officialStoppedStateIsKnownWithoutClaimingPositionsClosed() {
        val encoded = StrictJson.encode(OkxReadProjection.bot(row + mapOf("state" to "stopped", "stopType" to "2"), 7, 1800000000100, false))
        assertTrue(encoded.contains("\"status\":\"stopped\""))
        assertTrue(encoded.contains("\"status_reason\":null"))
        assertFalse(encoded.contains("closed")); assertFalse(encoded.contains("-13.375"))
    }
}
