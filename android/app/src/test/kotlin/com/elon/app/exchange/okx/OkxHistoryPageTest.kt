package com.elon.app.exchange.okx

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class OkxHistoryPageTest {
    private val credentials = OkxCredentials("key", "secret", "passphrase")
    private fun rows(vararg ids: String) = """{"code":"0","data":[${ids.joinToString(",") { """{"algoId":"$it","state":"stopped"}""" }}]}"""
    @Test fun fixedGetAndNumericOldestSupportNonemptyShortPages() {
        val requests = mutableListOf<OkxReadRequest>()
        val reader = OkxHistoryReader(OkxReadGateway { _, request -> requests.add(request); rows("101", "99") })
        val page = reader.read(credentials, "")
        assertEquals("99", page.nextAfter)
        assertEquals("/api/v5/tradingBot/grid/orders-algo-history?algoOrdType=contract_grid&instType=SWAP&limit=50", requests.single().path)
        assertEquals(2, page.rows.size)
        assertTrue(OkxReadRequest.History.of("99").path.endsWith("&after=99"))
        for (bad in listOf("0", "-1", "01", "1&limit=1000", "https://example.com"))
            assertThrows(IllegalArgumentException::class.java) { OkxReadRequest.History.of(bad) }
    }
    @Test fun onlySuccessfulEmptyPageIsTerminal() {
        val page = OkxHistoryReader(OkxReadGateway { _, _ -> rows() }).read(credentials, "99")
        assertNull(page.nextAfter)
        val encoded = StrictJson.parse(page.encode(OkxAccount("a".repeat(64), "sub"), 1, 2, 1000, emptyList()))
        assertEquals(OkxHistoryPage.SCHEMA, encoded["schema"])
        assertEquals("99", encoded["after"]); assertEquals(true, encoded["complete"])
        assertEquals(OkxReadFailure.RATE_LIMITED, assertThrows(OkxReadException::class.java) {
            OkxHistoryReader(OkxReadGateway { _, _ -> """{"code":"50011","data":[]}""" }).read(credentials, "99")
        }.reason)
    }
    @Test fun duplicateWrongDirectionAndUnexpectedStateAreRejected() {
        for (raw in listOf(rows("90", "90"), rows("101"), rows("100"), rows("90").replace("stopped", "running"))) {
            assertEquals(OkxReadFailure.INVALID_RESPONSE, assertThrows(OkxReadException::class.java) {
                OkxHistoryReader(OkxReadGateway { _, _ -> raw }).read(credentials, "100")
            }.reason)
        }
        assertEquals(OkxReadFailure.RESPONSE_LIMIT, assertThrows(OkxReadException::class.java) {
            OkxHistoryReader(OkxReadGateway { _, _ -> rows(*(1..51).map(Int::toString).toTypedArray()) }).read(credentials, "")
        }.reason)
    }
}
