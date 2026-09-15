package com.elon.app.grid

import java.net.URLEncoder

/** Keep Binance's exact official code; syntax never replaces catalogue or account validation. */
internal object BinanceSymbols {
    const val BASE = "[A-Z0-9\\u3400-\\u4DBF\\u4E00-\\u9FFF]{1,24}"
    const val PATTERN = BASE + "USDT"
    private val contract = Regex(PATTERN)
    fun valid(value:String)=contract.matches(value)
    fun encoded(value:String):String {
        require(valid(value))
        return URLEncoder.encode(value,"UTF-8")
    }
}
