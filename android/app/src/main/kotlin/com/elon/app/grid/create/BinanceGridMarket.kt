package com.elon.app.grid.create

import com.elon.app.privateaccess.StrictJson
import java.math.BigDecimal
import java.net.URL
import javax.net.ssl.HttpsURLConnection

internal class BinanceGridRule(val symbol: String, val tick: BigDecimal, val minimumQuantity: String, val minimumNotional: String?,
    val categories: List<String> = emptyList(), val quantityStep: String? = null,
    val priceMinimum: String? = null, val priceMaximum: String? = null, val pricePrecision: Int? = null) {
    fun validate(draft: BinanceGridDraft) {
        require(draft.symbol == symbol) { "合约已变化，请刷新行情" }
        for (key in listOf("lower", "upper")) require(BigDecimal(draft.input.getValue(key)).remainder(tick).signum() == 0) {
            "价格需要按 ${tick.stripTrailingZeros().toPlainString()} USDT 的最小变动单位填写"
        }
        for (key in listOf("triggerPrice", "stopLower", "stopUpper", "trailingUpPrice", "trailingDownPrice")) {
            draft.input[key]?.takeIf { it.isNotEmpty() }?.let { require(BigDecimal(it).remainder(tick).signum() == 0) { "高级价格需要符合最小价格变动单位" } }
        }
    }
}
internal class BinanceGridQuote(val symbol: String, val mark: String, val index: String, val funding: String, val observed: Long)

/** Public market data only; no cookies, API keys, trading calls or arbitrary URLs. */
internal class BinanceGridMarket {
    private var cached: List<BinanceGridRule> = emptyList()
    private var loadedAt = 0L
    fun rules(): List<BinanceGridRule> {
        if (cached.isNotEmpty() && System.currentTimeMillis() - loadedAt in 0 until 600_000) return cached
        val data = get("/fapi/v1/exchangeInfo", 4_194_304)
        cached = parseRules(data); loadedAt = System.currentTimeMillis(); return cached
    }
    fun quote(symbol: String): BinanceGridQuote {
        require(Regex("[A-Z0-9]{1,24}USDT").matches(symbol))
        return parseQuote(get("/fapi/v1/premiumIndex?symbol=$symbol", 16_384), symbol, System.currentTimeMillis())
    }
    fun tickers(): Map<String, BinanceSymbolTicker> = BinanceSymbolTicker.parse(get("/fapi/v1/ticker/24hr", 4_194_304), System.currentTimeMillis())
    fun ticker(symbol:String):BinanceSymbolTicker {
        require(Regex("[A-Z0-9]{1,24}USDT").matches(symbol))
        val raw=get("/fapi/v1/ticker/24hr?symbol=$symbol",16_384)
        return BinanceSymbolTicker.parse("[$raw]",System.currentTimeMillis())[symbol]?.also {require(it.last!=null)}
            ?: error("当前合约最新价不可用")
    }
    private fun get(path: String, maximum: Int): String {
        val connection = URL("https://fapi.binance.com$path").openConnection() as HttpsURLConnection
        try {
            connection.requestMethod = "GET"; connection.instanceFollowRedirects = false
            connection.connectTimeout = 8000; connection.readTimeout = 10000; connection.useCaches = false
            connection.setRequestProperty("Accept", "application/json")
            require(connection.responseCode == 200) { "行情连接未返回有效响应" }
            val bytes = java.io.ByteArrayOutputStream()
            connection.inputStream.use { input ->
                val buffer = ByteArray(8192)
                while (true) {
                    val count = input.read(buffer)
                    if (count < 0) break
                    require(bytes.size() + count <= maximum)
                    bytes.write(buffer, 0, count)
                }
            }
            return bytes.toString("UTF-8")
        } finally { connection.disconnect() }
    }
    companion object {
        fun parseRules(raw: String): List<BinanceGridRule> {
            val root = StrictJson.parse(raw, 4_194_304, maxArrayItems = 10000, maxNodes = 500000)
            val symbols = root["symbols"] as? List<*> ?: error("合约列表不可用")
            require(symbols.size <= 10_000)
            return symbols.mapNotNull { item ->
                @Suppress("UNCHECKED_CAST") val data = item as? Map<String, Any?> ?: return@mapNotNull null
                if (data["status"] != "TRADING" || data["contractType"] != "PERPETUAL" || data["quoteAsset"] != "USDT" || data["marginAsset"] != "USDT") return@mapNotNull null
                val symbol = data["symbol"] as? String ?: return@mapNotNull null
                if (!Regex("[A-Z0-9]{1,24}USDT").matches(symbol)) return@mapNotNull null
                @Suppress("UNCHECKED_CAST") val filters = (data["filters"] as? List<*>)?.mapNotNull { it as? Map<String, Any?> }.orEmpty()
                fun filter(type: String, key: String) = filters.find { it["filterType"] == type }?.get(key) as? String
                val tick = filter("PRICE_FILTER", "tickSize") ?: return@mapNotNull null
                val qty = filter("LOT_SIZE", "minQty") ?: return@mapNotNull null
                if (!positive(tick) || !positive(qty)) return@mapNotNull null
                val notional = filter("MIN_NOTIONAL", "notional")?.takeIf(::positive)
                val precision=(data["pricePrecision"] as? StrictJson.Number)?.text?.toIntOrNull()?.takeIf { it in 0..20 }
                BinanceGridRule(symbol, BigDecimal(tick), qty, notional, BinanceSymbolCatalog.tags(data["underlyingSubType"]),
                    filter("LOT_SIZE","stepSize")?.takeIf(::positive),filter("PRICE_FILTER","minPrice")?.takeIf(::positive),
                    filter("PRICE_FILTER","maxPrice")?.takeIf(::positive),precision)
            }.distinctBy { it.symbol }.sortedBy { it.symbol }.also { require(it.isNotEmpty()) { "暂未取得可交易的 U 本位永续合约" } }
        }
        fun parseQuote(raw: String, expected: String, now: Long): BinanceGridQuote {
            val data = StrictJson.parse(raw, 16_384)
            require(data["symbol"] == expected)
            fun decimal(key: String): String = (data[key] as? String)?.also {
                require(Regex("-?(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?").matches(it))
            } ?: error("行情数据不完整")
            val observed = (data["time"] as? StrictJson.Number)?.text?.toLong() ?: error("行情时间缺失")
            require(now - observed in -5000..120_000)
            val mark = decimal("markPrice"); val index = decimal("indexPrice")
            require(positive(mark) && positive(index))
            return BinanceGridQuote(expected, mark, index, decimal("lastFundingRate"), observed)
        }
        private fun positive(value: String) = Regex("(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?").matches(value) && BigDecimal(value).signum() > 0
    }
}
