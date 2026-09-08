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
    private fun list(vararg rows: Map<String, Any?>, account: String = "42", kind: String = "primary") =
        StrictJson.encode(mapOf("kind" to "list", "account" to account, "account_kind" to kind, "rows" to rows.toList()))
    private fun detail(row: Map<String, Any?>, account: String = "42") =
        StrictJson.encode(mapOf("kind" to "detail", "account" to account, "account_kind" to "primary", "row" to row))
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
    @Test fun continuousTicketsRenewButLegacyExpiredAndRevokedTicketsCannot() {
        val s = state(); s.accept(list(row()))
        val legacy = s.grant(); val continuous = s.grant(true)
        rejected { s.renew(legacy) }
        clock += 800_000; s.renew(continuous)
        clock += 100_001; assertFalse(s.authorized(legacy)); assertTrue(s.authorized(continuous))
        s.revoke(continuous); rejected { s.renew(continuous) }
        val expired = s.apply { accept(list(row())) }.grant(true)
        clock += 900_000; rejected { s.renew(expired) }
    }
    @Test fun enrichedDetailKeepsListMetricsAndLegacyWireUnchanged() {
        val s = state(); s.accept(list(row() + ("metrics" to mapOf("matchedPnl" to "1.23", "fee" to "0.01"))))
        val token = s.grant(true)
        s.accept(detail(row(account = null) + ("metrics" to mapOf("matchedPnl" to null, "closeOnStop" to true))))
        assertFalse(s.reply(token).contains("metrics"))
        val raw = s.reply(token, true)
        assertTrue(raw.contains("yilong.binance_host_read.v2"))
        assertTrue(raw.contains("\"matchedPnl\":\"1.23\"")); assertTrue(raw.contains("\"closeOnStop\":true"))
    }
    @Test fun accountChangeInvalidatesOldGrant() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        state.accept(list(row(account = "43"), account = "43")); assertFalse(state.authorized(grant)); assertTrue(state.fresh())
    }
    @Test fun sameAccountListRefreshKeepsGrant() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        state.accept(list(row("124"))); assertTrue(state.authorized(grant)); assertFalse(state.contains("123"))
    }
    @Test fun missingMixedAndUnverifiedAccountResponsesAreRejected() {
        rejected { state().accept("{\"kind\":\"list\",\"rows\":[]}") }
        rejected { state().accept(list(row(account = null))) }
        rejected { state().accept(list(row(), row("124", "43"))) }
        rejected { state().accept(list(row(), account = "43", kind = "sub")) }
    }
    @Test fun duplicateAndInvalidPriceResponsesAreRejected() {
        rejected { state().accept(list(row(), row())) }
        rejected { state().accept(list(row().apply { put("upper", "0.1") })) }
        rejected { state().accept(list(row().apply { put("profit", 2) })) }
    }
    @Test fun detailMustMatchObservedGridAndDoesNotRefreshListAge() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        state.accept(detail(row(account = null)))
        assertTrue(state.reply(grant).contains("\"detail\":true"))
        clock += 300_000
        assertFalse(state.fresh()); assertTrue(state.reply(grant).contains("\"rows\":[]"))
    }
    @Test fun invalidationDropsAllPrivateState() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        state.unavailable(); assertFalse(state.authorized(grant)); assertEquals(0, state.count)
        assertNull(state.account)
    }
    @Test fun verifiedEmptySubaccountDoesNotInheritPrimaryRowsOrGrant() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        state.accept(list(account = "43", kind = "sub"))
        assertFalse(state.authorized(grant)); assertTrue(state.fresh()); assertEquals(0, state.count)
        assertEquals("sub", state.accountKind); assertTrue(state.reply(state.grant()).contains("\"rows\":[]"))
    }
    @Test fun malformedResponseClearsPreviouslyGrantedDataInsideStateBoundary() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        rejected { state.accept(list(row(), account = "43")) }
        assertFalse(state.authorized(grant)); assertFalse(state.fresh()); assertEquals(0, state.count)
    }
    @Test fun observedIdentitySwitchClearsDataEvenWithoutAnotherGridResponse() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        state.accept("{\"kind\":\"identity\",\"account\":\"43\",\"account_kind\":\"sub\"}")
        assertFalse(state.authorized(grant)); assertFalse(state.fresh()); assertEquals(0, state.count)
        assertEquals("sub", state.accountKind)
    }
    @Test fun detailRequiresCurrentAccountProofAndCountsOnlyCurrentRows() {
        val state = state(); state.accept(list(row())); val grant = state.grant()
        assertEquals(0, state.detailCount); assertEquals(1, state.activeGrantCount)
        state.accept(detail(row(account = null))); assertEquals(1, state.detailCount)
        rejected { state.accept(detail(row(account = null), account = "43")) }
        assertEquals(0, state.detailCount); assertEquals(0, state.activeGrantCount)
        assertFalse(state.authorized(grant))
    }
    @Test fun navigationOnlyAllowsExactOfficialHttpsHosts() {
        assertTrue(binanceHostNavigation("https://www.binance.com/zh-CN/trading-bots"))
        assertTrue(binanceHostNavigation("https://accounts.binance.com/login"))
        listOf("https://www.binance.com.evil.test/", "http://www.binance.com/", "https://x@www.binance.com/", "https://www.binance.com:444/").forEach {
            assertFalse(binanceHostNavigation(it))
        }
    }
}
