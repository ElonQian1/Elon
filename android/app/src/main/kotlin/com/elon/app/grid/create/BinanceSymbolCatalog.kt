package com.elon.app.grid.create

import java.util.Locale

/** Selection metadata only. Membership always comes from the current exchange contract list. */
internal object BinanceSymbolCatalog {
    enum class Scope { ALL, FAVORITES, RECENT }
    enum class Sort { NAME, VOLUME, GAIN, LOSS }
    val priceFields = setOf("lower", "upper", "triggerPrice", "stopLower", "stopUpper", "trailingUpPrice", "trailingDownPrice", "leverage")
    fun resetOnChange(previous: String, next: String) = if (previous.trim().equals(next.trim(), ignoreCase = true)) emptySet() else priceFields
    fun valid(symbol: String) = Regex("[A-Z0-9]{1,24}USDT").matches(symbol)
    fun query(raw: String) = raw.uppercase(Locale.ROOT).filterNot { it.isWhitespace() || it in "/-_" }.take(64)
    fun tags(value: Any?): List<String> = (value as? List<*>)?.take(16)?.mapNotNull {
        (it as? String)?.takeIf { tag -> Regex("[A-Za-z][A-Za-z0-9 _-]{0,39}").matches(tag) }?.uppercase(Locale.ROOT)
    }?.distinct().orEmpty()
    fun label(tag: String): String = when (tag) {
        "AI" -> "AI"; "MEME" -> "Meme"; "DEFI" -> "DeFi"; "STORAGE" -> "存储"
        "LAYER1", "LAYER_1", "LAYER-1" -> "公链"; "LAYER2", "LAYER_2", "LAYER-2" -> "Layer 2"
        "GAMING", "GAMEFI" -> "游戏"; "NFT" -> "NFT"; "METAVERSE" -> "元宇宙"
        "INFRASTRUCTURE" -> "基础设施"; "PAYMENT" -> "支付"; "RWA" -> "RWA"; "" -> "未分类"
        else -> tag
    }
    fun recent(values: List<String>, symbol: String): List<String> =
        (listOf(symbol) + values).filter(::valid).distinct().take(12)
    fun favorites(values: Set<String>, symbol: String): Set<String> {
        val clean = values.filter(::valid).take(100).toMutableSet()
        if (symbol in clean) clean.remove(symbol) else if (valid(symbol) && clean.size < 100) clean.add(symbol)
        return clean
    }
    fun select(rules: List<BinanceGridRule>, search: String, scope: Scope, category: String?, sort: Sort,
        favorites: Set<String>, recent: List<String>, tickers: Map<String, BinanceSymbolTicker>, now: Long): List<BinanceGridRule> {
        val q = query(search)
        val rows = rules.filter { row ->
            (q.isEmpty() || row.symbol.contains(q)) && (category == null || if (category.isEmpty()) row.categories.isEmpty() else category in row.categories) &&
                when (scope) { Scope.ALL -> true; Scope.FAVORITES -> row.symbol in favorites; Scope.RECENT -> row.symbol in recent }
        }
        return rows.sortedWith { a, b ->
            val exact = (if (q.isNotEmpty() && a.symbol.removeSuffix("USDT") == q || a.symbol == q) 0 else 1)
                .compareTo(if (q.isNotEmpty() && b.symbol.removeSuffix("USDT") == q || b.symbol == q) 0 else 1)
            if (exact != 0) exact else {
                fun number(row: BinanceGridRule) = tickers[row.symbol]?.takeIf { it.fresh(now) }?.let {
                    when (sort) { Sort.VOLUME -> it.volume; Sort.GAIN, Sort.LOSS -> it.change; Sort.NAME -> null }
                }
                val x = number(a); val y = number(b)
                val order = when {
                    sort == Sort.NAME && scope == Scope.RECENT -> recent.indexOf(a.symbol).compareTo(recent.indexOf(b.symbol))
                    sort == Sort.NAME -> 0
                    x == null && y == null -> 0
                    x == null -> 1
                    y == null -> -1
                    sort == Sort.LOSS -> x.compareTo(y)
                    else -> y.compareTo(x)
                }
                if (order != 0) order else a.symbol.compareTo(b.symbol)
            }
        }
    }
}
