package com.elon.app.grid.create

import com.elon.app.privateaccess.StrictJson
import java.math.BigDecimal

internal data class BinanceSymbolTicker(val last: BigDecimal?, val change: BigDecimal?, val volume: BigDecimal?, val closeTime: Long) {
    fun fresh(now: Long) = now - closeTime in -5_000..120_000
    companion object {
        fun parse(raw: String, now: Long): Map<String, BinanceSymbolTicker> {
            require(raw.toByteArray(Charsets.UTF_8).size <= 4_194_304)
            val rows = StrictJson.parse("{\"rows\":$raw}", 4_194_320, 10000, 500000)["rows"] as? List<*> ?: error("行情列表不可用")
            val result = linkedMapOf<String, BinanceSymbolTicker>()
            for (item in rows) {
                val row = item as? Map<*, *> ?: continue
                val symbol = row["symbol"] as? String ?: continue
                if (!BinanceSymbolCatalog.valid(symbol)) continue
                val time = (row["closeTime"] as? StrictJson.Number)?.text?.toLongOrNull() ?: continue
                fun number(key: String, signed: Boolean = false): BigDecimal? {
                    val value = row[key] as? String ?: return null
                    if (!Regex("-?(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?").matches(value)) return null
                    return BigDecimal(value).takeIf { signed || it.signum() >= 0 }
                }
                val ticker = BinanceSymbolTicker(number("lastPrice")?.takeIf { it.signum() > 0 }, number("priceChangePercent", true), number("quoteVolume"), time)
                if (ticker.fresh(now)) result[symbol] = ticker
            }
            return result
        }
    }
}
