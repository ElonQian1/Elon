package com.elon.app.grid

import com.elon.app.grid.create.BinanceSymbolCatalog
import com.elon.app.grid.create.BinanceGridDraft
import org.junit.Assert.*
import org.junit.Test

class BinanceChineseSymbolsTest {
    @Test fun officialSpellingsAndEncodingRemainExact() {
        for(base in listOf("龙虾","币安人生","我踏马来了","牛来","哈基米","NEAR","1000SHIB")) {
            assertTrue(BinanceSymbolCatalog.valid(base+"USDT"))
            assertTrue(BinanceSymbols.valid(base+"USDT"))
        }
        assertEquals("%E9%BE%99%E8%99%BEUSDT",BinanceSymbols.encoded("龙虾USDT"))
    }
    @Test fun controlsDelimitersAndUnboundedCodesAreStillRejected() {
        for(value in listOf("龙虾USDT\n","龙虾 USDT","龙虾USDT&symbol=BTCUSDT","../龙虾USDT","龙虾%26USDT","龙虾\u200bUSDT","龙".repeat(25)+"USDT")) {
            assertFalse(value,BinanceSymbols.valid(value))
            assertTrue(runCatching{BinanceSymbols.encoded(value)}.isFailure)
        }
    }
}
