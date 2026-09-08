package com.elon.app.grid.host

/** Observed Binance fields. Missing values remain null; no profit or margin is invented. */
internal object BinanceGridMetrics {
    val decimalFields = setOf("initialNotional", "investment", "matchedPnl", "fundingFee", "fee",
        "adjustmentAmount", "perGridQty", "perGridQuoteQty", "triggerPrice", "stopUpper", "stopLower",
        "stopTpPnl", "stopSlPnl", "trailingUpPrice", "trailingDownPrice")
    val flags = setOf("closeOnStop", "autoAddMargin", "trailingUp", "trailingDown")
    val fields = decimalFields + flags + setOf("matchedCount", "marginType", "orderCurrency", "ended")
    fun decode(value: Any?): Map<String, Any?> {
        if (value == null) return emptyMap()
        @Suppress("UNCHECKED_CAST") val data = value as? Map<String, Any?> ?: error("METRICS_INVALID")
        require(data.keys.all { it in fields })
        data.forEach { (key, item) ->
            if (item != null) when (key) {
                in decimalFields -> require(item is String && Regex("-?(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?").matches(item))
                in flags -> require(item is Boolean)
                "marginType" -> require(item in setOf("CROSSED", "ISOLATED"))
                "orderCurrency" -> require(item in setOf("BASE", "QUOTE"))
                "matchedCount", "ended" -> require(item is String && Regex("0|[1-9][0-9]{0,15}").matches(item))
            }
        }
        return data
    }
}
