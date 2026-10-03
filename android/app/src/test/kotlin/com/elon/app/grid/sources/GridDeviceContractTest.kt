package com.elon.app.grid.sources

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class GridDeviceContractTest {
    private fun source(platform: String, account: String = "a".repeat(64)) = mapOf(
        "source_id" to (if (platform == "android") "c" else "d").repeat(64),
        "snapshot" to mapOf("schema" to GridDeviceContract.SCHEMA, "platform" to platform,
            "device_id" to "00000000-0000-4000-8000-000000000001", "sequence" to 2L,
            "account" to account, "account_kind" to "primary", "observed_at_ms" to 1000L,
            "fresh_until_ms" to 301000L, "status" to "fresh", "rows" to listOf(mapOf(
                "id" to "7", "symbol" to "BTCUSDT", "status" to "WORKING", "direction" to "LONG",
                "spacing" to "ARITH", "lower" to "10", "upper" to "20", "count" to "12",
                "leverage" to "2", "profit" to "-0.00000000000000000001", "created" to "900",
                "detail" to false, "metrics" to mapOf("fundingFee" to null, "autoAddMargin" to false)))))
    private fun raw(vararg sources: Map<String, Any?>) = StrictJson.encode(mapOf(
        "schema" to "yilong.grid_device_sources.v1", "sources" to sources.toList()))
    private fun reject(action: () -> Unit) { try { action(); fail("accepted invalid source") } catch (_: RuntimeException) {} }
    @Test fun bothSourcesAndSameStrategyIdsRemainIndependent() {
        val list = GridDeviceContract.parse(raw(source("android"), source("windows")), 1000)
        assertEquals(2, list.size); assertNotEquals(list[0].key("7"), list[1].key("7"))
        assertEquals("-0.00000000000000000001", list[0].rows[0]["profit"])
        assertEquals(2L, list[0].sequence)
    }
    @Test fun expiryUsesOriginalObservationNotReadTime() {
        val list = GridDeviceContract.parse(raw(source("windows")), 301000)
        assertFalse(list.single().fresh(301000)); assertTrue(list.single().fresh(300999))
    }
    @Test fun malformedUnknownAndDuplicateFieldsRejected() {
        val valid = raw(source("android"))
        reject { GridDeviceContract.parse(valid.replace("\"sequence\":2", "\"sequence\":2.0"), 1000) }
        reject { GridDeviceContract.parse(valid.replace("\"sequence\":2", "\"sequence\":2,\"sequence\":3"), 1000) }
        reject { GridDeviceContract.parse(valid.replace("\"metrics\":{", "\"metrics\":{\"cookie\":null,"), 1000) }
        reject { GridDeviceContract.parse(valid.replace("\"upper\":\"20\"", "\"upper\":\"9\""), 1000) }
        reject { GridDeviceContract.parse(raw(source("android"), source("android")), 1000) }
    }
}
