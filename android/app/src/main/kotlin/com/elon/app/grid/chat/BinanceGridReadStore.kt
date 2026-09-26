package com.elon.app.grid.chat

import com.elon.app.grid.host.BinanceGridMetrics

/** In-memory immutable snapshots. Context is local-only; replies contain no account or document IDs. */
internal class BinanceGridReadStore(private val elapsed: () -> Long) {
    data class Context(val owner: String, val account: String, val kind: String, val document: String)
    private data class Entry(val request: BinanceGridReadRequest, val started: Long,
        var context: Context? = null, var status: String = "pending", var error: String? = null,
        var observed: Long = 0, var rows: List<Map<String, String?>> = emptyList())
    private val entries = linkedMapOf<String, Entry>()
    fun contains(id: String) = entries.containsKey(id)
    fun pending(id: String) = entries[id]?.status == "pending"
    fun context(id: String) = entries[id]?.context
    fun start(request: BinanceGridReadRequest) {
        require(request.start && !contains(request.requestId))
        entries.entries.removeAll { elapsed() - it.value.started >= TTL }
        require(entries.size < 16) { "snapshot_capacity" }
        entries[request.requestId] = Entry(request, elapsed())
    }
    fun fail(id: String, error: String) {
        entries[id]?.let { it.status = "failed"; it.error = error; it.rows = emptyList() }
    }
    fun complete(id: String, context: Context, observed: Long, rows: List<Map<String, Any?>>) {
        val entry = entries[id] ?: return
        if (entry.status != "pending") return
        require(rows.size <= 500 && rows.map { it["id"] }.toSet().size == rows.size)
        if (entry.request.kind == "detail") require(rows.size == 1 && rows.single()["id"] == entry.request.strategyId)
        entry.rows = rows.map(::project); entry.context = context; entry.observed = observed; entry.status = "ready"
    }
    fun reply(request: BinanceGridReadRequest, current: Context?): Map<String, Any?> {
        val base = mapOf("schema" to "yilong.binance_grid_read.v1", "source" to "android_webview",
            "kind" to request.kind, "request_id" to request.requestId, "trading_enabled" to false)
        fun failed(error: String) = base + mapOf("status" to "failed", "error" to error)
        val entry = entries[request.requestId] ?: return failed("snapshot_not_found")
        if (entry.request.identity != request.identity) return failed("request_conflict")
        if (elapsed() - entry.started !in 0 until TTL) { fail(request.requestId, "snapshot_expired"); return failed("snapshot_expired") }
        if (entry.status == "ready" && entry.context != current) fail(request.requestId, "context_changed")
        if (entry.status == "failed") return failed(entry.error ?: "read_failed")
        if (entry.status == "pending") return base + ("status" to "pending")
        if (request.offset > entry.rows.size) return failed("invalid_offset")
        val common = base + mapOf("status" to "ready", "observed_at_ms" to entry.observed,
            "valid_for_ms" to (TTL - (elapsed() - entry.started)), "coverage" to "observed_response_only")
        if (request.kind == "detail") return common + ("row" to entry.rows.single())
        val end = (request.offset + request.limit).coerceAtMost(entry.rows.size)
        return common + mapOf("total" to entry.rows.size, "offset" to request.offset,
            "next_offset" to end.takeIf { it < entry.rows.size }, "rows" to entry.rows.subList(request.offset, end))
    }
    companion object {
        const val TTL = 300_000L
        val baseFields = setOf("id", "symbol", "status", "direction", "spacing", "lower", "upper", "count", "leverage", "profit", "created")
        val metricFields = setOf("investment", "initialNotional", "perGridQty", "perGridQuoteQty", "matchedPnl",
            "fundingFee", "fee", "matchedCount", "marginType", "orderCurrency", "stopUpper", "stopLower")
        fun project(row: Map<String, Any?>): Map<String, String?> {
            val metrics = BinanceGridMetrics.decode(row["metrics"])
            require(row["id"] is String && Regex("[1-9][0-9]{0,19}").matches(row["id"] as String))
            require(row["symbol"] is String && com.elon.app.grid.BinanceSymbols.valid(row["symbol"] as String))
            return (baseFields + metricFields).associateWith { key ->
                (if (key in baseFields) row[key] else metrics[key]) as? String
            }
        }
    }
}
