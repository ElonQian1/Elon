package com.elon.app.grid.host

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceHostStateTest {
    private var clock = 1_000L
    private val epoch = 1_788_800_000_000L
    private fun state() = BinanceHostState({ clock }, { epoch })
    private fun row(id: String = "123", account: String? = "42") = linkedMapOf<String, Any?>(
        "id" to id, "account" to account, "symbol" to "NEARUSDT", "status" to "WORKING",
        "direction" to "LONG", "spacing" to "ARITH", "lower" to "1.2", "upper" to "2.4",
        "count" to "12", "leverage" to "2", "profit" to "-0.15", "created" to "1788790000000")
    private fun list(vararg rows: Map<String, Any?>) = StrictJson.encode(mapOf("kind" to "list", "rows" to rows.toList()))
    private fun rejected(action: () -> Unit) { try { action(); fail("accepted invalid input") } catch (_: RuntimeException) {} }

    @Test fun validReadExcludesAccountAndCarriesLocalSource() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        val raw = state.reply(grant); val result = StrictJson.parse(raw)
        assertEquals("android_webview", result["source"])
        assertFalse(raw.contains("account")); assertFalse(raw.contains(grant))
        assertEquals(1, state.count)
    }
    @Test fun grantUsesMonotonicExpiryAndCanBeRevoked() {
        val state = state(); state.accept(list(row())); val first = state.grant(); val second = state.grant()
        state.revoke(first); assertFalse(state.authorized(first)); assertTrue(state.authorized(second))
        clock += 900_000; assertFalse(state.authorized(second)); rejected { state.reply(second) }
    }
    @Test fun accountChangeInvalidatesOldGrant() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        state.accept(list(row(account = "43"))); assertFalse(state.authorized(grant)); assertTrue(state.fresh())
    }
    @Test fun sameAccountListRefreshKeepsGrant() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        state.accept(list(row("124"))); assertTrue(state.authorized(grant)); assertFalse(state.contains("123"))
    }
    @Test fun missingMixedAndEmptyAccountResponsesAreRejected() {
        rejected { state().accept(list()) }
        rejected { state().accept(list(row(account = null))) }
        rejected { state().accept(list(row(), row("124", "43"))) }
    }
    @Test fun duplicateAndInvalidPriceResponsesAreRejected() {
        rejected { state().accept(list(row(), row())) }
        rejected { state().accept(list(row().apply { put("upper", "0.1") })) }
        rejected { state().accept(list(row().apply { put("profit", 2) })) }
    }
    @Test fun detailMustMatchObservedGridAndDoesNotRefreshListAge() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        state.accept(StrictJson.encode(mapOf("kind" to "detail", "row" to row(account = null))))
        assertTrue(state.reply(grant).contains("\"detail\":true"))
        clock += 300_000
        assertFalse(state.fresh()); assertTrue(state.reply(grant).contains("\"rows\":[]"))
    }
    @Test fun invalidationDropsAllPrivateState() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        state.unavailable(); assertFalse(state.authorized(grant)); assertEquals(0, state.count)
    }
    @Test fun navigationOnlyAllowsExactOfficialHttpsHosts() {
        assertTrue(binanceHostNavigation("https://www.binance.com/zh-CN/trading-bots"))
        assertTrue(binanceHostNavigation("https://accounts.binance.com/login"))
        listOf("https://www.binance.com.evil.test/", "http://www.binance.com/", "https://x@www.binance.com/", "https://www.binance.com:444/").forEach {
            assertFalse(binanceHostNavigation(it))
        }
    }
}
