package com.elon.app.grid.create

import com.elon.app.privateaccess.StrictJson
import java.math.BigDecimal
import org.junit.Assert.*
import org.junit.Test

class BinanceGridMarketTest {
    private fun symbol(name: String = "NEARUSDT", status: String = "TRADING", contract: String = "PERPETUAL") =
        """{"symbol":"$name","status":"$status","contractType":"$contract","quoteAsset":"USDT","marginAsset":"USDT","filters":[{"filterType":"PRICE_FILTER","tickSize":"0.001"},{"filterType":"LOT_SIZE","minQty":"1"},{"filterType":"MIN_NOTIONAL","notional":"5"}]}"""
    @Test fun excludesUnavailableContractsAndAcceptsCatalogLargerThanPrivatePayloadLimit() {
        val many = (1..501).map { symbol("A${it}USDT") } + symbol(status = "SETTLING") + symbol(contract = "CURRENT_QUARTER")
        val raw = "{\"symbols\":[${many.joinToString(",")}]}"
        val rules = BinanceGridMarket.parseRules(raw)
        assertEquals(501, rules.size); assertEquals(BigDecimal("0.001"), rules.first().tick)
        try { StrictJson.parse(raw); fail("private wire limit must stay bounded") } catch (_: IllegalArgumentException) {}
    }
    @Test fun validatesExactTickWithoutFloatingPointRounding() {
        val draft = mapOf("symbol" to "NEARUSDT", "direction" to "LONG", "spacing" to "ARITH", "marginType" to "ISOLATED",
            "lower" to "1.001", "upper" to "2", "margin" to "100", "leverage" to "2", "count" to "10", "autoInit" to "true", "closeOnStop" to "true")
        val rule = BinanceGridMarket.parseRules("{\"symbols\":[${symbol()}]}").single()
        rule.validate(BinanceGridDraft.parse(draft))
        for (change in listOf(mapOf("lower" to "1.0001"), mapOf("triggerPrice" to "1.0001"), mapOf("symbol" to "BTCUSDT"))) {
            try { rule.validate(BinanceGridDraft.parse(draft + change)); fail("incorrect contract or price tick accepted") } catch (_: IllegalArgumentException) {}
        }
    }
    @Test fun catalogUsesExchangeClassificationWithoutBreakingLegacyMissingTags() {
        val tagged = symbol().replace("\"filters\":", "\"underlyingSubType\":[\"STORAGE\",\"AI\"],\"filters\":")
        assertEquals(listOf("STORAGE", "AI"), BinanceGridMarket.parseRules("{\"symbols\":[$tagged]}").single().categories)
        assertTrue(BinanceGridMarket.parseRules("{\"symbols\":[${symbol()}]}").single().categories.isEmpty())
    }
    @Test fun quoteRequiresExpectedContractFreshTimeAndPositivePrices() {
        fun quote(symbol: String = "NEARUSDT", time: Long = 1000000, mark: String = "1.234") =
            """{"symbol":"$symbol","markPrice":"$mark","indexPrice":"1.23","lastFundingRate":"-0.00001","time":$time}"""
        assertEquals("-0.00001", BinanceGridMarket.parseQuote(quote(), "NEARUSDT", 1000000).funding)
        for (raw in listOf(quote(symbol = "BTCUSDT"), quote(time = 800000), quote(time = 1100000), quote(mark = "0"))) {
            try { BinanceGridMarket.parseQuote(raw, "NEARUSDT", 1000000); fail("invalid quote accepted") } catch (_: IllegalArgumentException) {}
        }
    }
}
