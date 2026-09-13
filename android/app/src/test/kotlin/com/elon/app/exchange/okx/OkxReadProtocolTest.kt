package com.elon.app.exchange.okx

import org.junit.Assert.*
import org.junit.Test

class OkxReadProtocolTest {
    @Test fun fixedRequestsRejectInjectedIdsAndPreserveQueryOrder() {
        assertEquals("/api/v5/tradingBot/grid/orders-algo-pending?algoOrdType=contract_grid&instType=SWAP&limit=100&after=981", OkxReadRequest.Pending.after("981").path)
        assertTrue(OkxReadRequest.Detail.of("981").path.endsWith("&algoId=981"))
        for (id in listOf("0", "01", "1&method=POST", "../a", "", "9".repeat(65)))
            assertThrows(IllegalArgumentException::class.java) { OkxReadRequest.Detail.of(id) }
        assertEquals("2020-12-08T09:08:57.715Z", OkxReadProtocol.timestamp(1607418537715))
        assertEquals("cVjwEzWyNEueAGiM0MKSC5AMY6R5BYz5sdoQ8mv/kbQ=", OkxReadProtocol.signature(
            "test-secret", "2020-12-08T09:08:57.715Z", OkxReadRequest.Account))
        assertNotEquals(OkxReadProtocol.signature("test-secret", "time", OkxReadRequest.Pending.first()),
            OkxReadProtocol.signature("test-secret", "time", OkxReadRequest.Pending.after("981")))
    }
    @Test fun credentialsAndAccountAreNeverPrinted() {
        val credentials = OkxCredentials("test-key", "test-secret", "test-passphrase")
        assertFalse(credentials.toString().contains(credentials.secret))
        assertThrows(IllegalArgumentException::class.java) { OkxCredentials("test-key\r\nCookie:a", "a", "b") }
    }
    @Test fun accountIsBoundToActualUidAndOnlyReadPermission() {
        fun raw(uid: String, main: String, perm: String) = """{"code":"0","data":[{"uid":"$uid","mainUid":"$main","perm":"$perm"}]}"""
        val primary = OkxReadProtocol.account(raw("10", "10", "read_only"))
        val sub = OkxReadProtocol.account(raw("11", "10", "read_only"))
        assertEquals("primary", primary.kind); assertEquals("sub", sub.kind); assertNotEquals(primary.reference, sub.reference)
        assertEquals(64, primary.reference.length)
        for (perm in listOf("trade", "read_only,trade", "read_only,withdraw", "")) {
            val error = assertThrows(OkxReadException::class.java) { OkxReadProtocol.account(raw("10", "10", perm)) }
            assertEquals(OkxReadFailure.READ_ONLY_KEY_REQUIRED, error.reason)
        }
    }
    @Test fun upstreamErrorAndDuplicateJsonDoNotBecomeAnEmptyList() {
        assertEquals(OkxReadFailure.RATE_LIMITED, assertThrows(OkxReadException::class.java) {
            OkxReadProtocol.rows("""{"code":"50011","data":[]}""", 100)
        }.reason)
        assertThrows(IllegalArgumentException::class.java) { OkxReadProtocol.rows("""{"code":"0","code":"1","data":[]}""", 100) }
    }
}
