package com.elon.app.grid.host

/** Closed projection: source response extras and account identifiers never cross the APK boundary. */
internal object BinanceReportFields {
    private const val DECIMAL = "-?(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?"
    private const val INTEGER = "(0|[1-9][0-9]{0,19})"
    private const val ENUM = "[A-Z][A-Z0-9_]{0,63}"
    private const val SYMBOL = "[A-Z0-9]{1,24}USDT"
    private val history = mapOf("id" to INTEGER, "symbol" to SYMBOL, "status" to ENUM, "direction" to "LONG|SHORT|NEUTRAL",
        "lower" to DECIMAL, "upper" to DECIMAL, "count" to INTEGER, "leverage" to INTEGER, "profit" to DECIMAL,
        "matchedPnl" to DECIMAL, "fundingFee" to DECIMAL, "fee" to DECIMAL, "created" to INTEGER, "end" to INTEGER,
        "investment" to DECIMAL, "initialNotional" to DECIMAL)
    private val orders = mapOf("side" to "BUY|SELL", "price" to DECIMAL, "quantity" to DECIMAL, "executed" to DECIMAL, "status" to ENUM, "time" to INTEGER)
    private val matches = mapOf("sequence" to "-?$INTEGER", "time" to INTEGER, "profit" to DECIMAL, "asset" to ENUM)
    private val deals = mapOf("time" to INTEGER, "side" to "BUY|SELL", "type" to ENUM, "price" to DECIMAL,
        "quantity" to DECIMAL, "total" to DECIMAL, "fee" to DECIMAL, "feeAsset" to ENUM)
    private val positions = mapOf("symbol" to SYMBOL, "quantity" to DECIMAL, "entry" to DECIMAL, "isolatedWallet" to DECIMAL)
    fun decode(kind: String, value: Any?): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST") val row = value as? Map<String, Any?> ?: error("REPORT_ROW_INVALID")
        val fields = when(kind) { "history" -> history; "orders" -> orders; "matches" -> matches; "deals" -> deals; "positions" -> positions; else -> error("REPORT_KIND") }
        val extra = when(kind) { "matches" -> setOf("details", "cancelled"); "positions" -> setOf("isolated"); else -> emptySet() }
        require(row.keys == fields.keys + extra)
        fields.forEach { (key, pattern) -> require(row[key] == null || row[key] is String && Regex(pattern).matches(row[key] as String)) }
        when(kind) {
            "history" -> require(listOf("id", "symbol", "status").all { row[it] is String })
            "orders", "deals" -> require(row["side"] is String)
            "matches" -> {
                require(row["sequence"] is String && row["cancelled"] is Boolean)
                val details = row["details"] as? List<*> ?: error("REPORT_DETAILS")
                require(details.size <= 2); details.forEach { decode("deals", it) }
            }
            "positions" -> require(row["symbol"] is String && (row["isolated"] == null || row["isolated"] is Boolean))
        }
        return row
    }
}
