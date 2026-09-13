package com.elon.app.exchange.okx

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

internal object OkxRecordFixtures {
    const val ID = "123"
    const val SYMBOL = "BTC-USDT-SWAP"
    fun order(id: String = "90", state: String = "live") = mapOf<String, Any?>("algoId" to ID, "algoOrdType" to "contract_grid", "instId" to SYMBOL,
        "instType" to "SWAP", "ordId" to id, "side" to "buy", "state" to state, "ordType" to "limit", "posSide" to "net", "px" to "60100.00",
        "sz" to "3", "accFillSz" to if(state=="filled")"3" else "1", "avgPx" to "60100", "fee" to "-0.12", "feeCcy" to "USDT", "rebate" to "0.01", "rebateCcy" to "USDT")
    fun position() = mapOf<String, Any?>("algoId" to ID, "instId" to SYMBOL, "instType" to "SWAP", "posSide" to "net", "pos" to "-3.00",
        "avgPx" to "60100", "ccy" to "USDT", "mgnMode" to "cross", "imr" to "25.00", "upl" to "-2.5")
    fun response(rows: List<Map<String, Any?>>) = StrictJson.encode(mapOf("code" to "0", "data" to rows))
    fun reader(rows: List<Map<String, Any?>>) = OkxRecordsReader(OkxReadGateway { _, request ->
        response(if(request is OkxReadRequest.Detail)listOf(mapOf("algoId" to ID,"algoOrdType" to "contract_grid","instId" to SYMBOL)) else rows)
    })
    val credentials = OkxCredentials("fixture-key", "fixture-secret", "fixture-passphrase")
}

class OkxRecordsPageTest {
    private fun query(kind: String = "orders", after: String = "") = OkxRecordQuery(kind, OkxRecordFixtures.ID, OkxRecordFixtures.SYMBOL, after)
    @Test fun closedRequestsRejectUnknownKindsIdentifiersAndPositionCursors() {
        assertEquals("/api/v5/tradingBot/grid/sub-orders?algoOrdType=contract_grid&algoId=123&type=live&limit=50&after=99", OkxReadRequest.Records.of(query(after="99")).path)
        assertTrue(OkxReadRequest.Records.of(query("fills")).path.contains("type=filled"))
        assertEquals("/api/v5/tradingBot/grid/positions?algoOrdType=contract_grid&algoId=123", OkxReadRequest.Records.of(query("positions")).path)
        for (kind in listOf("write","matches","")) assertThrows(IllegalArgumentException::class.java){query(kind)}
        assertThrows(IllegalArgumentException::class.java){query("positions","99")}
        assertThrows(IllegalArgumentException::class.java){OkxRecordQuery("orders","123&x=1",OkxRecordFixtures.SYMBOL,"")}
        assertThrows(IllegalArgumentException::class.java){OkxRecordQuery("orders","123","BTC-USD-SWAP","")}
    }
    @Test fun shortPagesAreNotTheEndAndFieldsKeepOriginalSignsAndMissingValues() {
        val page = OkxRecordFixtures.reader(listOf(OkxRecordFixtures.order())).read(OkxRecordFixtures.credentials, query(), 1800000000100L)
        assertEquals("90",page.nextAfter);assertFalse(page.complete)
        assertEquals("60100",page.rows.single()["price"]);assertEquals("-0.12",page.rows.single()["fee"])
        assertEquals("0.01",page.rows.single()["rebate"]);assertNull(page.rows.single()["updated"])
        val empty=OkxRecordFixtures.reader(emptyList()).read(OkxRecordFixtures.credentials,query(after="90"),1800000000100L)
        assertTrue(empty.complete);assertNull(empty.nextAfter)
        val positions=OkxRecordFixtures.reader(listOf(OkxRecordFixtures.position())).read(OkxRecordFixtures.credentials,query("positions"),1800000000100L)
        assertTrue(positions.complete);assertEquals("-3",positions.rows.single()["quantity"]);assertNull(positions.rows.single()["liquidation_price"])
    }
    @Test fun rejectsWrongStrategyInstrumentCursorDuplicatesAndOversizedPages() {
        for(rows in listOf(listOf(OkxRecordFixtures.order()+mapOf("algoId" to "124")),listOf(OkxRecordFixtures.order()+mapOf("instId" to "ETH-USDT-SWAP")),
            listOf(OkxRecordFixtures.order(),OkxRecordFixtures.order()),(1..51).map{OkxRecordFixtures.order(it.toString())},
            listOf(OkxRecordFixtures.order()+mapOf("accFillSz" to "4")))) {
            assertThrows(Exception::class.java){OkxRecordFixtures.reader(rows).read(OkxRecordFixtures.credentials,query(),1800000000100L)}
        }
        assertThrows(IllegalArgumentException::class.java){OkxRecordFixtures.reader(listOf(OkxRecordFixtures.order())).read(OkxRecordFixtures.credentials,query(after="80"),1800000000100L)}
        assertThrows(IllegalArgumentException::class.java){OkxRecordFixtures.reader(listOf(OkxRecordFixtures.order())).read(OkxRecordFixtures.credentials,query("fills"),1800000000100L)}
        assertThrows(IllegalArgumentException::class.java){OkxRecordFixtures.reader(listOf(OkxRecordFixtures.position(),OkxRecordFixtures.position())).read(OkxRecordFixtures.credentials,query("positions"),1800000000100L)}
    }
    @Test fun emptyRowsDoNotBypassTheActualStrategyIdentityCheck() {
        val reader=OkxRecordsReader(OkxReadGateway { _,_->OkxRecordFixtures.response(listOf(mapOf("algoId" to "999","algoOrdType" to "contract_grid","instId" to OkxRecordFixtures.SYMBOL)))})
        assertThrows(IllegalArgumentException::class.java){reader.read(OkxRecordFixtures.credentials,query(),1800000000100L)}
    }
}
