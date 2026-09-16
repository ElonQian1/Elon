package com.elon.app.grid.host

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceGridReportsTest {
    @Test fun richPositionsAreVersionedAndLegacyFieldsRemainExact() {
        val row=mapOf("symbol" to "NEARUSDT","quantity" to "-2.1234567890123456789","entry" to "3",
            "isolatedWallet" to "0","isolated" to false,"positionMargin" to "2","leverage" to "10","pnl" to "-0.1")
        val state=BinanceGridReports({100},{1000000});val account=BinanceHostState.digest("42")
        state.start(query(mapOf("kind" to "positions","id" to "123","symbol" to "NEARUSDT")),account,"sub")
        state.accept(event(mapOf("kind" to "positions","total" to 1,"coverage" to "strategy_position","rows" to listOf(row))))
        val old=StrictJson.parse(state.reply(request,account,"sub"));val rich=StrictJson.parse(state.reply(request,account,"sub",2))
        assertEquals("yilong.binance_report.v1",old["schema"]);assertEquals("yilong.binance_report.v2",rich["schema"])
        val legacy=(old["rows"] as List<*>).single() as Map<*,*>
        assertEquals(setOf("symbol","quantity","entry","isolatedWallet","isolated"),legacy.keys)
        assertEquals(row,(rich["rows"] as List<*>).single())
        assertThrows(IllegalArgumentException::class.java){BinanceReportFields.decode("positions",row+("walletToken" to "private"))}
    }
    private val request = "a".repeat(32)
    private fun query(changes: Map<String, Any?> = emptyMap()) = BinanceReportQuery.parse(StrictJson.encode(mapOf(
        "request" to request, "kind" to "history", "id" to "", "symbol" to "", "page" to 1, "days" to 30) + changes))
    private fun event(changes: Map<String, Any?> = emptyMap()) = StrictJson.parse(StrictJson.encode(mapOf("schema" to "yilong.binance_report_observation.v1",
        "token" to "doc_test_123", "request" to request, "kind" to "history", "account" to "42", "account_kind" to "sub",
        "status" to "ready", "page" to 1, "total" to 0, "coverage" to "page", "rows" to emptyList<Any>()) + changes))
    @Test fun requestIsClosedAndBoundedToKnownKindsAndPages() {
        for(change in listOf(mapOf("url" to "/close-grid"), mapOf("kind" to "close"), mapOf("days" to 365), mapOf("page" to 1001), mapOf("id" to "123"))) {
            assertThrows(IllegalArgumentException::class.java) { query(change) }
        }
    }
    @Test fun identityMismatchAndReusedRequestAreRejected() {
        val state = BinanceGridReports({100},{1000000}); state.start(query(), BinanceHostState.digest("42"), "sub")
        assertThrows(IllegalArgumentException::class.java) { state.accept(event(mapOf("account" to "43"))) }
        assertThrows(IllegalArgumentException::class.java) { state.start(query(), BinanceHostState.digest("42"), "sub") }
        state.accept(event()); val raw = state.reply(request, BinanceHostState.digest("42"), "sub")
        assertEquals("ready", StrictJson.parse(raw)["status"]); assertFalse(raw.contains("account"))
        assertThrows(IllegalArgumentException::class.java) { state.reply(request, BinanceHostState.digest("43"), "sub") }
    }
    @Test fun staleOrLateReportsCannotStayOnScreen() {
        var elapsed = 100L
        val state = BinanceGridReports({elapsed},{1000000}); state.start(query(), BinanceHostState.digest("42"), "sub")
        elapsed += 35001; state.accept(event())
        assertEquals("error", StrictJson.parse(state.reply(request, BinanceHostState.digest("42"), "sub"))["status"])
        state.clear(); state.start(query(), BinanceHostState.digest("42"), "sub"); state.accept(event())
        elapsed += 300001
        val data = StrictJson.parse(state.reply(request, BinanceHostState.digest("42"), "sub")); assertEquals("error", data["status"])
        assertEquals(emptyList<Any>(), data["rows"])
    }
    @Test fun extraRowFieldsAndFloatingPointAmountsNeverLeaveTheHost() {
        val row = mapOf("side" to "BUY", "price" to "1.234567890123456789", "quantity" to "1", "executed" to "0", "status" to "NEW", "time" to null)
        assertEquals(row, BinanceReportFields.decode("orders", row))
        assertThrows(IllegalArgumentException::class.java) { BinanceReportFields.decode("orders", row + ("account" to "42")) }
        assertThrows(IllegalArgumentException::class.java) { BinanceReportFields.decode("orders", row + ("price" to 1.2)) }
    }
}
