package com.elon.app.exchange.okx

import org.junit.Assert.*
import org.junit.Test

class OkxPendingReaderTest {
    private val credentials = OkxCredentials("test-key", "test-secret", "test-passphrase")
    private fun page(vararg ids: String) = """{"code":"0","data":[${ids.joinToString(",") { """{"algoId":"$it"}""" }}]}"""
    @Test fun shortUnsortedPagesContinueByNumericOldestUntilEmpty() {
        val requests = mutableListOf<String>()
        val pages = ArrayDeque(listOf(page("101", "99"), page("98"), page()))
        val gateway = OkxReadGateway { _, request -> requests.add(request.path); pages.removeFirst() }
        val rows = OkxPendingReader(gateway) { 0 }.read(credentials)
        assertEquals(listOf("101", "99", "98"), rows.map { it["algoId"] })
        assertEquals(3, requests.size); assertTrue(requests[1].endsWith("after=99")); assertTrue(requests[2].endsWith("after=98"))
    }
    @Test fun overlappingPageCannotBecomeSuccessfulPartialInventory() {
        val pages = ArrayDeque(listOf(page("99"), page("99")))
        assertEquals(OkxReadFailure.INVALID_RESPONSE, assertThrows(OkxReadException::class.java) {
            OkxPendingReader(OkxReadGateway { _, _ -> pages.removeFirst() }) { 0 }.read(credentials)
        }.reason)
    }
    @Test fun failedLaterPageDoesNotReturnEarlierBots() {
        var calls = 0
        assertThrows(OkxReadException::class.java) {
            OkxPendingReader(OkxReadGateway { _, _ -> if (++calls == 1) page("99") else """{"code":"50011","data":[]}""" }) { 0 }.read(credentials)
        }
        assertEquals(2, calls)
    }
    @Test fun deadlineAndPageCapDoNotPretendCompletion() {
        var clock = 0L
        assertThrows(OkxReadException::class.java) {
            OkxPendingReader(OkxReadGateway { _, _ -> clock = 60_001; page() }) { clock }.read(credentials)
        }
        var id = 999
        assertEquals(OkxReadFailure.RESPONSE_LIMIT, assertThrows(OkxReadException::class.java) {
            OkxPendingReader(OkxReadGateway { _, _ -> page((id--).toString()) }) { 0 }.read(credentials)
        }.reason)
    }
}
