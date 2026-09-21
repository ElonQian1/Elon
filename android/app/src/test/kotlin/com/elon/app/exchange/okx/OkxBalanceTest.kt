package com.elon.app.exchange.okx

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class OkxBalanceTest {
    private fun read(details: String) = OkxBalance.decode("""{"code":"0","data":[{"totalEq":"999","details":$details}]}""")
    @Test fun usesUsdtFieldsWithoutConvertingAccountUsdTotal() {
        val values = read("""[{"ccy":"BTC","eq":"5"},{"ccy":"USDT","eq":"12.123456789012345678","availBal":"0","frozenBal":"2.1"}]""")
        assertEquals("12.123456789012345678", values["equity"])
        assertEquals("0", values["available"])
        assertEquals("2.1", values["frozen"])
    }
    @Test fun missingAndBlankAreUnknownNotZero() {
        assertTrue(read("[]").values.all { it == null })
        assertTrue(read("""[{"ccy":"USDT","eq":"","availBal":null}]""").values.all { it == null })
    }
    @Test fun duplicatesAndInvalidAmountsAreRejected() {
        listOf("""[{"ccy":"USDT"},{"ccy":"USDT"}]""", """[{"ccy":"USDT","eq":1}]""",
            """[{"ccy":"USDT","availBal":"NaN"}]""").forEach { details ->
            assertThrows(IllegalArgumentException::class.java) { read(details) }
        }
    }
    @Test fun accountAndObservationAreBoundToReceipt() {
        val value = StrictJson.parse(OkxBalance.encode(OkxAccount("a".repeat(64), "sub"), 7, 1234, read("[]")))
        assertEquals("sub", value["account_kind"]); assertEquals("USDT", value["quote_asset"])
        assertEquals(OkxBalance.SCHEMA, value["schema"])
    }
}
