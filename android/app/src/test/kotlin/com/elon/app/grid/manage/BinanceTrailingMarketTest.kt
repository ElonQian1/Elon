package com.elon.app.grid.manage

import com.elon.app.grid.create.BinanceGridQuote
import com.elon.app.grid.create.BinanceGridRule
import com.elon.app.grid.create.BinanceSymbolTicker
import java.math.BigDecimal
import org.junit.Assert.*
import org.junit.Test

class BinanceTrailingMarketTest {
    private val rule get()=BinanceGridRule("TESTUSDT",BigDecimal("0.01"),"0.1",null,
        quantityStep="0.1",priceMinimum="0.01",priceMaximum="10000",pricePrecision=2)
    private val quote get()=BinanceGridQuote("TESTUSDT","7","7","0.001",100000)
    private val ticker get()=BinanceSymbolTicker(BigDecimal("7.01"),null,null,100001)
    @Test fun freshContextBindsTheTwoObservedTimesAndExactRules() {
        val v=BinanceTrailingMarket.context(rule,quote,ticker,100010)
        assertEquals("TESTUSDT",v["symbol"]);assertEquals("7.01",v["last"])
        assertEquals("0.1",v["quantity_step"]);assertEquals(2,v["price_precision"])
        assertEquals(100000L,v["observed_at"]);assertEquals(10,v.size)
    }
    @Test fun missingMetadataStaleMarketAndAnotherSymbolCannotBePrepared() {
        for(block in listOf<()->Unit>(
            {BinanceTrailingMarket.context(BinanceGridRule("TESTUSDT",BigDecimal("0.01"),"0.1",null),quote,ticker,100010)},
            {BinanceTrailingMarket.context(rule,BinanceGridQuote("OTHERUSDT","7","7","0",100000),ticker,100010)},
            {BinanceTrailingMarket.context(rule,quote,ticker,120001)},
            {BinanceTrailingMarket.context(rule,quote,ticker,94999)},
            {BinanceTrailingMarket.context(rule,quote,ticker.copy(last=null),100010)}
        )) assertTrue(runCatching(block).isFailure)
    }
}
