package com.elon.app.grid.host

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceStrategyFundsTest {
    private val request = "b".repeat(32)
    private val account = BinanceHostState.digest("42")
    private val row = mapOf("asset" to "USDT", "marginBalance" to "1.234567890123456789", "crossInitialMargin" to null)
    private fun query(page: Int = 1) = BinanceReportQuery.parse(StrictJson.encode(mapOf("request" to request,
        "kind" to "funds", "id" to "123", "symbol" to "NEARUSDT", "page" to page, "days" to 30)))
    private fun event(rows: List<Map<String, Any?>> = listOf(row), owner: String = "42") = StrictJson.parse(StrictJson.encode(mapOf(
        "schema" to "yilong.binance_report_observation.v1", "token" to "doc_fixture", "request" to request, "kind" to "funds",
        "account" to owner, "account_kind" to "sub", "status" to "ready", "page" to 1,
        "total" to rows.size, "coverage" to "strategy_margin", "rows" to rows)))
    @Test fun capabilityAndQueryAreExplicitAndSinglePage() {
        val capability = StrictJson.parse(BinanceReportQuery.capabilities())
        assertEquals(setOf("schema", "kinds"), capability.keys)
        assertEquals("yilong.binance_report_capabilities.v1", capability["schema"])
        assertTrue((capability["kinds"] as List<*>).contains("funds"))
        assertEquals("funds", query().kind)
        assertThrows(IllegalArgumentException::class.java) { query(2) }
    }
    @Test fun fundsAreExactScopedAndRequireExactlyOneRow() {
        val reports = BinanceGridReports({100}, {1000000}); reports.start(query(), account, "sub")
        assertThrows(IllegalArgumentException::class.java) { reports.accept(event(emptyList())) }
        assertThrows(IllegalArgumentException::class.java) { reports.accept(event(listOf(row, row))) }
        assertThrows(IllegalArgumentException::class.java) { reports.accept(event(owner = "43")) }
        reports.accept(event())
        val reply = StrictJson.parse(reports.reply(request, account, "sub"))
        assertEquals(listOf(row), reply["rows"]); assertFalse(reply.containsKey("account"))
        assertThrows(IllegalArgumentException::class.java) { reports.reply(request, account, "primary") }
        reports.clear()
        assertThrows(IllegalStateException::class.java) { reports.reply(request, account, "sub") }
    }
    @Test fun fundsRejectWrongUnitsPrivateFieldsAndFloatingPointValues() {
        assertEquals(row, BinanceReportFields.decode("funds", row))
        for (change in listOf(mapOf("asset" to "BTC"), mapOf("asset" to null), mapOf("owner" to "42"), mapOf("marginBalance" to 1.25))) {
            assertThrows(IllegalArgumentException::class.java) { BinanceReportFields.decode("funds", row + change) }
        }
        assertEquals("0", BinanceReportFields.decode("funds", row + ("marginBalance" to "0"))["marginBalance"])
    }
}
