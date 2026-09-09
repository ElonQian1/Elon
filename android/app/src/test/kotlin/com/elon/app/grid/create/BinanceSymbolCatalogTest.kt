package com.elon.app.grid.create

import org.junit.Assert.*
import org.junit.Test
import java.math.BigDecimal

class BinanceSymbolCatalogTest {
    private val now = 1_000_000L
    private fun rule(symbol: String, vararg tags: String) = BinanceGridRule(symbol, BigDecimal("0.001"), "1", "5", tags.toList())
    private val rules = listOf(rule("NEARUSDT", "LAYER1"), rule("1000NEARUSDT", "MEME"), rule("BTCUSDT"), rule("ASTORUSDT", "STORAGE"))
    private fun select(search: String = "", scope: BinanceSymbolCatalog.Scope = BinanceSymbolCatalog.Scope.ALL,
        category: String? = null, sort: BinanceSymbolCatalog.Sort = BinanceSymbolCatalog.Sort.NAME,
        favorites: Set<String> = emptySet(), recent: List<String> = emptyList(), quotes: Map<String, BinanceSymbolTicker> = emptyMap()) =
        BinanceSymbolCatalog.select(rules, search, scope, category, sort, favorites, recent, quotes, now).map { it.symbol }
    private fun quote(volume: String?, change: String?, time: Long = now) = BinanceSymbolTicker(BigDecimal.ONE, change?.let(::BigDecimal), volume?.let(::BigDecimal), time)
    @Test fun searchNormalizesSymbolsAndRanksExactBaseBeforeSubstring() {
        assertEquals(listOf("NEARUSDT", "1000NEARUSDT"), select("near"))
        assertEquals(listOf("BTCUSDT"), select(" btc / usdt "))
        assertTrue(select("UNKNOWN").isEmpty())
    }
    @Test fun classificationIsSourceDrivenAndUnknownIsSeparate() {
        assertEquals(listOf("ASTORUSDT"), select(category = "STORAGE"))
        assertEquals(listOf("BTCUSDT"), select(category = ""))
        assertTrue(select(category = "AI").isEmpty())
        assertEquals(listOf("AI", "STORAGE"), BinanceSymbolCatalog.tags(listOf("ai", "STORAGE", "AI", 3, "x\n", "<script>")))
        assertTrue(BinanceSymbolCatalog.tags("AI").isEmpty())
    }
    @Test fun scopesNeverCreateRowsMissingFromLiveCatalogAndPreserveRecentOrder() {
        assertEquals(listOf("BTCUSDT"), select(scope = BinanceSymbolCatalog.Scope.FAVORITES, favorites = setOf("BTCUSDT", "FAKEUSDT")))
        assertEquals(listOf("NEARUSDT", "BTCUSDT"), select(scope = BinanceSymbolCatalog.Scope.RECENT, recent = listOf("NEARUSDT", "BTCUSDT", "OLDUSDT")))
    }
    @Test fun sortingPreservesDecimalPrecisionAndPlacesStaleOrMissingAfterKnown() {
        val quotes = mapOf("NEARUSDT" to quote("9007199254740993.0000000001", "-1"), "BTCUSDT" to quote("9007199254740993.0000000002", "2"),
            "ASTORUSDT" to quote("999999999999999999", "99", now - 120001))
        assertEquals(listOf("BTCUSDT", "NEARUSDT"), select(sort = BinanceSymbolCatalog.Sort.VOLUME, quotes = quotes).take(2))
        assertEquals(listOf("NEARUSDT", "BTCUSDT"), select(sort = BinanceSymbolCatalog.Sort.LOSS, quotes = quotes).take(2))
        assertEquals("BTCUSDT", select(sort = BinanceSymbolCatalog.Sort.GAIN, quotes = quotes).first())
    }
    @Test fun preferencesAreBoundedAndRecentlyChosenDoesNotDuplicate() {
        assertEquals(listOf("NEARUSDT", "BTCUSDT"), BinanceSymbolCatalog.recent(listOf("BTCUSDT", "NEARUSDT", "bad"), "NEARUSDT"))
        assertEquals(12, BinanceSymbolCatalog.recent((1..50).map { "A${it}USDT" }, "NEARUSDT").size)
        val full = (1..100).map { "A${it}USDT" }.toSet()
        assertEquals(full, BinanceSymbolCatalog.favorites(full, "BTCUSDT"))
        assertEquals(99, BinanceSymbolCatalog.favorites(full, "A1USDT").size)
    }
    @Test fun tickerDropsStaleRecordsAndKeepsMissingValuesUnknown() {
        val raw = """[{"symbol":"NEARUSDT","lastPrice":"1.00000000000000000001","priceChangePercent":"-1.2","quoteVolume":"9007199254740993.1","closeTime":1000000},
            {"symbol":"BTCUSDT","lastPrice":"0","priceChangePercent":1.5,"quoteVolume":"-5","closeTime":1000000},
            {"symbol":"OLDUSDT","closeTime":1},{"symbol":"BTCBUSD","closeTime":1000000}]"""
        val parsed = BinanceSymbolTicker.parse(raw, now)
        assertEquals(setOf("NEARUSDT", "BTCUSDT"), parsed.keys)
        assertEquals(BigDecimal("1.00000000000000000001"), parsed.getValue("NEARUSDT").last)
        assertNull(parsed.getValue("BTCUSDT").last); assertNull(parsed.getValue("BTCUSDT").change); assertNull(parsed.getValue("BTCUSDT").volume)
        assertFalse(parsed.getValue("NEARUSDT").fresh(now + 120001))
    }
    @Test fun selectingSameSymbolKeepsDraftButChangingSymbolClearsPriceAndLeverage() {
        assertTrue(BinanceSymbolCatalog.resetOnChange("NEARUSDT", " nearusdt ").isEmpty())
        val draft = mapOf("lower" to "1", "upper" to "2", "triggerPrice" to "1.5", "leverage" to "3", "margin" to "100")
        val cleared = draft - BinanceSymbolCatalog.resetOnChange("NEARUSDT", "BTCUSDT")
        assertEquals(mapOf("margin" to "100"), cleared)
    }
}
