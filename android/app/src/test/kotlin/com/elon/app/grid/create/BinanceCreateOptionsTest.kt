package com.elon.app.grid.create

import org.junit.Assert.*
import org.junit.Test

class BinanceCreateOptionsTest {
    private fun input() = mapOf("symbol" to "NEARUSDT", "direction" to "LONG", "spacing" to "ARITH", "marginType" to "ISOLATED",
        "lower" to "1", "upper" to "2", "margin" to "100", "leverage" to "2", "count" to "10", "autoInit" to "true", "closeOnStop" to "true")
    @Test fun neutralOmitsTheDirectionOnlyInitialPositionSwitch() {
        val draft = BinanceGridDraft.parse(input() + ("direction" to "NEUTRAL"))
        assertFalse(draft.payload().containsKey("autoInitPos")); assertTrue(draft.summary().contains("中性"))
    }
    @Test fun trailingAndStopsFollowManualConstructorWithoutInventedDefaults() {
        val draft = BinanceGridDraft.parse(input() + BinanceCreateOptions.defaults + mapOf("trailingUp" to "true", "trailingUpPrice" to "3", "stopLower" to "0.8", "triggerPrice" to "1.1"))
        val payload = draft.payload()
        assertEquals("QUOTE", payload["orderCurrency"]); assertEquals(true, payload["trailingStopLowerLimit"])
        assertEquals(false, payload["trailingStopUpperLimit"]); assertEquals("0.8", payload["stopLowerLimit"])
        assertEquals("MARK_PRICE", payload["triggerType"]); assertEquals(false, payload["autoAddMargin"])
        assertEquals(true, payload["tpslCps"])
    }
    @Test fun optionalPricesAndTrailingDependenciesRejectInvalidInputs() {
        for (options in listOf(mapOf("triggerPrice" to "-1"), mapOf("triggerPrice" to "0"), mapOf("stopLower" to "2", "stopUpper" to "1"),
            mapOf("trailingUpPrice" to "3"))) {
            try { BinanceGridDraft.parse(input() + options); fail("invalid option accepted") } catch (_: IllegalArgumentException) {}
        }
    }
    @Test fun roiBecomesPositiveAmountsWithTheSourceRoundingDirections() {
        val values = input() + mapOf("margin" to "123.456", "stopMode" to "ROI", "stopProfit" to "12.345", "stopLoss" to "12.345")
        val payload = BinanceGridDraft.parse(values).payload()
        assertEquals("15.24", payload["stopTpPnl"]); assertEquals("15.25", payload["stopSlPnl"])
        assertFalse(payload.containsKey("stopLowerLimit"))
        assertThrows(IllegalArgumentException::class.java) { BinanceGridDraft.parse(values + ("stopLower" to "1")) }
        assertThrows(IllegalArgumentException::class.java) { BinanceGridDraft.parse(values + ("stopLoss" to "101")).payload() }
    }
}
