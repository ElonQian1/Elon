package com.elon.app.grid.host

import com.elon.app.privateaccess.StrictJson
import org.junit.Assert.*
import org.junit.Test

class BinanceHostDiagnosticsTest {
    private fun event(): Map<String, Any?> = mapOf(
        "schema" to "yilong.binance_diagnostic.v1", "token" to "doc_test_123",
        "list_requests" to 1L, "list_responses" to 1L, "identity_requests" to 1L, "identity_responses" to 1L,
        "detail_requests" to 0L, "detail_responses" to 0L, "legacy_list_requests" to 0L, "script_errors" to 0L, "script_load_errors" to 0L,
        "script_count" to 2L, "last_failure" to "account_mismatch", "failure_kind" to "list",
        "last_http_status" to 200L, "route" to "grid", "ready_state" to "complete", "body_text_length" to 100L,
        "login_control" to false, "running_control" to true)

    @Test fun validFactsAreProjectedWithoutDocumentToken() {
        val state = BinanceHostDiagnostics()
        assertTrue(state.accept(StrictJson.parse(StrictJson.encode(event()))))
        assertEquals("account_mismatch", state.facts["last_failure"])
        assertEquals(1L, state.facts["list_requests"])
        assertFalse(state.facts.containsKey("token")); assertFalse(state.facts.containsKey("schema"))
        assertTrue(state.observedAt > 0)
    }
    @Test fun privateExtrasUnknownMessagesAndInvalidCountsAreRejected() {
        val invalid = listOf(event() + ("cookie" to "private-canary"), event() + ("last_failure" to "private-canary"),
            event() + ("failure_kind" to "https://private.invalid/"), event() + ("script_errors" to -1L),
            event() + ("list_requests" to 10001L), event() + ("last_http_status" to 600L),
            event() + ("body_text_length" to 1000001L), event() + ("list_requests" to "1"),
            event() + ("list_requests" to 1.0), event() + ("login_control" to "true"),
            event() + ("list_requests" to StrictJson.Number("1e0")),
            event() + ("list_requests" to StrictJson.Number("01")),
            event() + ("list_requests" to StrictJson.Number("-0")))
        invalid.forEach {
            val state = BinanceHostDiagnostics(); assertTrue(state.accept(event()))
            assertFalse(state.accept(it)); assertTrue(state.facts.isEmpty()); assertEquals(0L, state.observedAt)
        }
    }
    @Test fun resetDoesNotRetainPriorPageFacts() {
        val state = BinanceHostDiagnostics(); assertTrue(state.accept(event()))
        state.clear(); assertTrue(state.facts.isEmpty()); assertEquals(0L, state.observedAt)
    }
}
