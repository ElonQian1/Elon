package com.elon.app.grid.host

import com.elon.app.privateaccess.StrictJson
import com.elon.app.privateaccess.number

internal class BinanceReportQuery private constructor(val data: Map<String, Any?>) {
    val request get() = data["request"] as String
    val kind get() = data["kind"] as String
    val id get() = data["id"] as String
    val symbol get() = data["symbol"] as String
    val page get() = data.number("page")
    fun json() = StrictJson.encode(data)
    companion object {
        val kinds = setOf("history", "orders", "matches", "positions", "funds")
        fun capabilities() = StrictJson.encode(mapOf("schema" to "yilong.binance_report_capabilities.v1", "kinds" to kinds.toList()))
        fun parse(raw: String): BinanceReportQuery {
            val q = StrictJson.parse(raw, 2048)
            require(q.keys == setOf("request", "kind", "page", "days", "symbol", "id"))
            require(q["request"] is String && Regex("[0-9a-f]{32}").matches(q["request"] as String))
            require(q["kind"] in kinds)
            require(q.number("page") in 1..1000 && q.number("days") in setOf(7L,30L,90L))
            val id = q["id"] as? String ?: error("REPORT_SCOPE")
            val symbol = q["symbol"] as? String ?: error("REPORT_SCOPE")
            if (q["kind"] == "history") require(id.isEmpty() && (symbol.isEmpty() || Regex("[A-Z0-9]{1,24}USDT").matches(symbol)))
            else {
                require(Regex("[0-9]{1,20}").matches(id) && Regex("[A-Z0-9]{1,24}USDT").matches(symbol))
                require(q["kind"] == "matches" || q.number("page") == 1L)
            }
            return BinanceReportQuery(q)
        }
    }
}

internal class BinanceGridReports(private val elapsed: () -> Long, private val epoch: () -> Long) {
    private var query: BinanceReportQuery? = null
    private var account: String? = null
    private var accountKind: String? = null
    private var started = 0L
    private var observed = 0L
    private var result: Map<String, Any?>? = null
    private val histories = linkedMapOf<String, String>()
    fun clear() { query = null; account = null; accountKind = null; result = null; histories.clear() }
    fun contains(id: String, symbol: String) = histories[id] == symbol
    fun start(next: BinanceReportQuery, owner: String, kind: String) {
        if (account != owner || accountKind != kind) clear()
        require(query?.request != next.request) { "REPORT_REQUEST_REUSED" }
        account = owner; accountKind = kind; query = next; started = elapsed(); observed = 0; result = null
    }
    fun fail(request: String) {
        val q = query ?: return
        if (q.request == request) result = mapOf("status" to "error", "total" to 0, "coverage" to "unavailable", "rows" to emptyList<Any>())
    }
    fun accept(event: Map<String, Any?>) {
        val q = query ?: return
        if (event["request"] != q.request || elapsed() - started !in 0..35000) return
        require(event.keys == setOf("schema", "token", "request", "kind", "account", "account_kind", "status", "page", "total", "coverage", "rows"))
        require(event["kind"] == q.kind && event.number("page") == q.page)
        require(event["account"] is String && BinanceHostState.digest(event["account"] as String) == account && event["account_kind"] == accountKind)
        require(event["status"] in setOf("ready", "error"))
        val coverage = event["coverage"] as? String ?: error("REPORT_COVERAGE")
        val allowed = when(q.kind) { "history", "matches" -> setOf("page"); "positions" -> setOf("strategy_position"); "funds" -> setOf("strategy_margin"); else -> setOf("grid_slots", "open_orders", "limited_grid_slots", "limited_open_orders") }
        require(coverage in allowed || coverage == "unavailable" && event["status"] == "error")
        val rows = event["rows"] as? List<*> ?: error("REPORT_ROWS")
        require(rows.size <= if (q.kind in setOf("history", "matches")) 20 else 500)
        val decoded = rows.map { BinanceReportFields.decode(q.kind, it) }
        val total = event.number("total"); require(total in decoded.size.toLong()..10000000L)
        if (q.kind == "funds" && event["status"] == "ready") require(rows.size == 1 && total == 1L)
        if (event["status"] == "error") require(rows.isEmpty() && total == 0L && coverage == "unavailable")
        if (q.kind == "history" && event["status"] == "ready") {
            require(decoded.map { it["id"] }.toSet().size == decoded.size)
            if (histories.size + decoded.size > 2000) histories.clear()
            decoded.forEach { histories[it["id"] as String] = it["symbol"] as String }
        }
        result = mapOf("status" to event["status"], "total" to total, "coverage" to coverage, "rows" to decoded)
        observed = epoch()
    }
    fun reply(request: String, owner: String?, kind: String): String {
        val q = query ?: error("REPORT_MISSING")
        require(request == q.request && owner != null && account == owner && kind == accountKind)
        val age = elapsed() - started
        val value = if (age in 0..300000 && result != null) result!! else mapOf("status" to if (age in 0..35000) "pending" else "error",
            "total" to 0, "coverage" to "unavailable", "rows" to emptyList<Any>())
        return StrictJson.encode(mapOf("schema" to "yilong.binance_report.v1", "request" to q.request, "kind" to q.kind,
            "page" to q.page, "observed_at_ms" to observed) + value)
    }
}
