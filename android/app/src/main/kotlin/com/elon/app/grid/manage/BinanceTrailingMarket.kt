package com.elon.app.grid.manage

import com.elon.app.grid.create.BinanceGridMarket
import com.elon.app.grid.create.BinanceGridQuote
import com.elon.app.grid.create.BinanceGridRule
import com.elon.app.grid.create.BinanceSymbolTicker
import java.math.BigDecimal
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

/** Public inputs only. Account data and private headers never enter this worker. */
internal class BinanceTrailingMarket {
    private val market=BinanceGridMarket()
    fun read(symbol:String):Map<String,Any> {
        val rule=market.rules().find{it.symbol==symbol} ?: error("当前合约规则不可用")
        val quote=market.quote(symbol)
        val ticker=market.tickers()[symbol] ?: error("当前合约最新价不可用")
        return context(rule,quote,ticker,System.currentTimeMillis())
    }
    companion object {
        val worker=ThreadPoolExecutor(1,1,30,TimeUnit.SECONDS,ArrayBlockingQueue(1),
            { job -> Thread(job,"binance-trailing-market").apply{isDaemon=true} },ThreadPoolExecutor.AbortPolicy())
        fun context(rule:BinanceGridRule,quote:BinanceGridQuote,ticker:BinanceSymbolTicker,now:Long):Map<String,Any> {
            require(rule.symbol==quote.symbol && Regex("[A-Z0-9]{1,24}USDT").matches(rule.symbol))
            require(now-quote.observed in -5000..20000 && now-ticker.closeTime in -5000..20000) {"行情已过期，请重新读取后检查"}
            fun price(value:String?):String {
                val raw=value ?: error("币安尚未返回完整合约规则")
                require(Regex("(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?").matches(raw) && BigDecimal(raw).signum()>0)
                return BigDecimal(raw).stripTrailingZeros().toPlainString()
            }
            val minimum=price(rule.priceMinimum);val maximum=price(rule.priceMaximum)
            require(BigDecimal(minimum)<BigDecimal(maximum))
            val precision=rule.pricePrecision ?: error("币安尚未返回合约价格精度")
            require(precision in 0..20)
            return linkedMapOf("symbol" to rule.symbol,"tick" to price(rule.tick.toPlainString()),"min_quantity" to price(rule.minimumQuantity),
                "min_price" to minimum,"max_price" to maximum,"quantity_step" to price(rule.quantityStep),"price_precision" to precision,
                "mark" to price(quote.mark),"last" to price(ticker.last?.toPlainString()),"observed_at" to minOf(quote.observed,ticker.closeTime))
        }
    }
}
